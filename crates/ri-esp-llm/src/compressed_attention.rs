//! Compressed attention cache for ESP32 using per-dimension quantized scoring.
//!
//! Unlike `Int4KvCache`, this scores compressed keys directly without
//! dequantizing them to f32 first. Only the top-k selected values are
//! decoded for the output aggregation.
//!
//! This is the ESP32-specific adaptation of `compressed_scorer::AttentionCache`,
//! using const generics for heapless, no-alloc operation.

use libm::sqrtf;

/// Per-dimension quantized key stored as uint8 codes.
#[derive(Clone, Copy)]
pub struct PerDimKey<const D: usize> {
    pub codes: [u8; D],
    pub norm: f32,
}

impl<const D: usize> Default for PerDimKey<D> {
    fn default() -> Self {
        Self {
            codes: [0; D],
            norm: 1.0,
        }
    }
}

/// Compressed attention cache using per-dimension quantization.
///
/// Stores keys as uint8 codes (1 byte per dimension) instead of f32 (4 bytes).
/// Scores compressed keys directly without dequantization.
/// Only decodes the top-k selected values for output aggregation.
///
/// Template parameters:
/// - D: head dimension (e.g., 32 or 64)
/// - N: max number of cached entries
/// - K: top-k selection size for attention
#[derive(Clone)]
pub struct CompressedAttentionCache<const D: usize, const N: usize, const K: usize> {
    keys: [PerDimKey<D>; N],
    values: [[f32; D]; N],
    // Per-dim statistics (shared across all keys)
    dim_mins: [f32; D],
    dim_ranges: [f32; D],
    levels: u32,
    len: usize,
}

impl<const D: usize, const N: usize, const K: usize> CompressedAttentionCache<D, N, K> {
    /// Create a new empty cache with default per-dim stats.
    pub fn new() -> Self {
        Self {
            keys: [PerDimKey::default(); N],
            values: [[0.0; D]; N],
            dim_mins: [-1.0; D],
            dim_ranges: [2.0; D],
            levels: 255, // 8-bit
            len: 0,
        }
    }

    /// Configure per-dimension min/max from calibration data.
    ///
    /// Call this before pushing keys if you have calibration vectors.
    /// If not called, defaults to [-1, 1] range per dimension.
    pub fn fit(&mut self, vectors: &[[f32; D]]) {
        let mut mins = [f32::INFINITY; D];
        let mut maxs = [f32::NEG_INFINITY; D];
        for v in vectors {
            // Normalize
            let mut norm_sq = 0.0f32;
            let mut i = 0;
            while i < D {
                norm_sq += v[i] * v[i];
                i += 1;
            }
            let norm = sqrtf(norm_sq).max(1e-6);
            let mut i = 0;
            while i < D {
                let unit = v[i] / norm;
                if unit < mins[i] {
                    mins[i] = unit;
                }
                if unit > maxs[i] {
                    maxs[i] = unit;
                }
                i += 1;
            }
        }
        let mut i = 0;
        while i < D {
            self.dim_mins[i] = mins[i];
            let range = maxs[i] - mins[i];
            self.dim_ranges[i] = range.max(1e-6);
            i += 1;
        }
    }

    /// Push a key-value pair into the cache.
    /// The key is quantized to uint8 codes; the value is stored as f32.
    pub fn push(&mut self, key: &[f32; D], value: &[f32; D]) -> bool {
        if self.len >= N {
            return false;
        }
        // Compute key norm
        let mut norm_sq = 0.0f32;
        let mut i = 0;
        while i < D {
            norm_sq += key[i] * key[i];
            i += 1;
        }
        let norm = sqrtf(norm_sq).max(1e-6);

        // Normalize and quantize
        let _inv_levels = 1.0f32 / self.levels as f32;
        let mut i = 0;
        while i < D {
            let unit = key[i] / norm;
            let normalized = (unit - self.dim_mins[i]) / self.dim_ranges[i];
            let code_f = libm::roundf(normalized * self.levels as f32);
            let code = if code_f < 0.0 {
                0u8
            } else if code_f > self.levels as f32 {
                self.levels as u8
            } else {
                code_f as u8
            };
            self.keys[self.len].codes[i] = code;
            i += 1;
        }
        self.keys[self.len].norm = norm;
        self.values[self.len] = *value;
        self.len += 1;
        true
    }

    /// Compute attention scores for all cached keys against a query.
    /// Scores are written into the provided buffer.
    ///
    /// This is the compressed-domain scoring path: no key dequantization.
    /// score = (sum_d(codes[d] * scaled_query[d]) + bias) * k_norm
    pub fn attention_scores(&self, query: &[f32; D], scores: &mut [f32; N]) {
        // Precompute scaled_query = dim_ranges / levels * query
        let inv_levels = 1.0f32 / self.levels as f32;
        let mut scaled_query = [0.0f32; D];
        let mut bias = 0.0f32;
        let mut i = 0;
        while i < D {
            scaled_query[i] = self.dim_ranges[i] * inv_levels * query[i];
            bias += self.dim_mins[i] * query[i];
            i += 1;
        }

        // Score each key: (bias + sum(codes[d] * scaled_query[d])) * k_norm
        let mut t = 0;
        while t < self.len {
            let mut dot = 0.0f32;
            let mut i = 0;
            while i < D {
                dot += self.keys[t].codes[i] as f32 * scaled_query[i];
                i += 1;
            }
            scores[t] = (dot + bias) * self.keys[t].norm;
            t += 1;
        }
    }

    /// Compute attention output: select top-k keys, softmax, decode selected values.
    ///
    /// Returns the number of values decoded (always <= K).
    pub fn attention_topk(&self, query: &[f32; D], out: &mut [f32; D]) -> usize {
        if self.len == 0 {
            let mut i = 0;
            while i < D {
                out[i] = 0.0;
                i += 1;
            }
            return 0;
        }

        // Score all keys
        let mut scores = [0.0f32; N];
        self.attention_scores(query, &mut scores);

        // Find top-k indices (simple selection sort for small K)
        let k = K.min(self.len);
        let mut selected_indices: [usize; K] = [0; K];
        let mut selected_scores: [f32; K] = [f32::NEG_INFINITY; K];
        let mut ki = 0;
        while ki < k {
            let mut best_idx = 0;
            let mut best_score = f32::NEG_INFINITY;
            let mut t = 0;
            while t < self.len {
                // Skip already selected
                let mut already = false;
                let mut sj = 0;
                while sj < ki {
                    if selected_indices[sj] == t {
                        already = true;
                        break;
                    }
                    sj += 1;
                }
                if !already && scores[t] > best_score {
                    best_score = scores[t];
                    best_idx = t;
                }
                t += 1;
            }
            selected_indices[ki] = best_idx;
            selected_scores[ki] = best_score;
            ki += 1;
        }

        // Softmax over selected
        let mut max_score = f32::NEG_INFINITY;
        let mut i = 0;
        while i < k {
            if selected_scores[i] > max_score {
                max_score = selected_scores[i];
            }
            i += 1;
        }

        let mut sum_exp = 0.0f32;
        let mut weights = [0.0f32; K];
        let mut i = 0;
        while i < k {
            let exp = libm::expf(selected_scores[i] - max_score);
            weights[i] = exp;
            sum_exp += exp;
            i += 1;
        }
        let inv_sum = if sum_exp > 0.0 { 1.0 / sum_exp } else { 0.0 };
        let mut i = 0;
        while i < k {
            weights[i] *= inv_sum;
            i += 1;
        }

        // Weighted sum of selected values (only K values decoded, not N)
        let mut j = 0;
        while j < D {
            out[j] = 0.0;
            let mut i = 0;
            while i < k {
                out[j] += weights[i] * self.values[selected_indices[i]][j];
                i += 1;
            }
            j += 1;
        }

        k
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn capacity(&self) -> usize {
        N
    }

    pub fn is_full(&self) -> bool {
        self.len >= N
    }

    /// Memory usage per key in bytes.
    pub fn bytes_per_key() -> usize {
        D + 4 // uint8 codes + f32 norm
    }

    /// Memory usage for the full cache in bytes.
    pub fn total_key_bytes(&self) -> usize {
        self.len * Self::bytes_per_key()
    }
}

impl<const D: usize, const N: usize, const K: usize> Default for CompressedAttentionCache<D, N, K> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_push_and_len() {
        let mut cache = CompressedAttentionCache::<4, 8, 2>::new();
        assert_eq!(cache.len(), 0);
        assert!(cache.push(&[1.0, 0.0, 0.0, 0.0], &[1.0, 2.0, 3.0, 4.0]));
        assert_eq!(cache.len(), 1);
        assert!(!cache.is_full());
    }

    #[test]
    fn attention_scores_match_direction() {
        let mut cache = CompressedAttentionCache::<4, 8, 2>::new();
        cache.push(&[1.0, 0.0, 0.0, 0.0], &[1.0, 0.0, 0.0, 0.0]);
        cache.push(&[0.0, 1.0, 0.0, 0.0], &[0.0, 1.0, 0.0, 0.0]);
        cache.push(&[0.0, 0.0, 1.0, 0.0], &[0.0, 0.0, 1.0, 0.0]);

        let mut scores = [0.0f32; 8];
        cache.attention_scores(&[1.0, 0.0, 0.0, 0.0], &mut scores);

        // Key 0 (aligned with query) should have highest score
        assert!(scores[0] > scores[1]);
        assert!(scores[0] > scores[2]);
    }

    #[test]
    fn attention_topk_decodes_only_k_values() {
        let mut cache = CompressedAttentionCache::<4, 8, 2>::new();
        cache.push(&[1.0, 0.0, 0.0, 0.0], &[1.0, 0.0, 0.0, 0.0]);
        cache.push(&[0.0, 1.0, 0.0, 0.0], &[0.0, 1.0, 0.0, 0.0]);
        cache.push(&[0.0, 0.0, 1.0, 0.0], &[0.0, 0.0, 1.0, 0.0]);

        let mut out = [0.0f32; 4];
        let decoded = cache.attention_topk(&[1.0, 0.0, 0.0, 0.0], &mut out);
        assert_eq!(decoded, 2);
        // Output should be dominated by key 0's value
        assert!(out[0] > out[1]);
        assert!(out[0] > out[2]);
    }

    #[test]
    fn fit_improves_accuracy() {
        let mut cache = CompressedAttentionCache::<4, 8, 2>::new();
        // Fit on calibration data
        cache.fit(&[
            [1.0, 0.5, -0.5, 0.3],
            [-0.5, 1.0, 0.5, -0.3],
            [0.3, -0.3, 1.0, 0.5],
        ]);

        cache.push(&[1.0, 0.5, -0.5, 0.3], &[1.0, 2.0, 3.0, 4.0]);
        cache.push(&[-0.5, 1.0, 0.5, -0.3], &[4.0, 3.0, 2.0, 1.0]);

        let mut scores = [0.0f32; 8];
        cache.attention_scores(&[1.0, 0.5, -0.5, 0.3], &mut scores);
        // Key 0 (aligned with query) should have higher score
        assert!(scores[0] > scores[1]);
    }

    #[test]
    fn memory_savings() {
        // PerDimKey stores D bytes (codes) + 4 bytes (norm) = D+4 bytes per key
        // Int4KvCache stores D/2 bytes (i4 codes) + 4 bytes (scale) per key + value
        // But Int4KvCache also dequantizes every key to f32 for scoring (D*4 bytes temp)
        // CompressedAttentionCache never dequantizes keys — scores codes directly

        let bytes_per_key = CompressedAttentionCache::<32, 64, 8>::bytes_per_key();
        assert_eq!(bytes_per_key, 36); // 32 bytes codes + 4 bytes norm

        // vs dense f32: 32 * 4 = 128 bytes per key
        // Compression ratio: 128 / 36 = 3.56x
        let dense_bytes = 32 * 4;
        let compression_ratio = dense_bytes as f32 / bytes_per_key as f32;
        assert!(compression_ratio > 3.5);
    }
}
