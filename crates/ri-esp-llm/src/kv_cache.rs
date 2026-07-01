use crate::int4::{dequantize_i4, quantize_to_i4};

#[derive(Clone, Copy)]
pub struct KvEntry<const D: usize, const P: usize> {
    pub key: [u8; P],
    pub value: [u8; P],
    pub key_scale: f32,
    pub value_scale: f32,
}

impl<const D: usize, const P: usize> Default for KvEntry<D, P> {
    fn default() -> Self {
        Self {
            key: [0; P],
            value: [0; P],
            key_scale: 1.0,
            value_scale: 1.0,
        }
    }
}

pub struct Int4KvCache<const D: usize, const P: usize, const N: usize> {
    entries: [KvEntry<D, P>; N],
    len: usize,
}

impl<const D: usize, const P: usize, const N: usize> Int4KvCache<D, P, N> {
    pub fn new() -> Self {
        debug_assert!(P >= D.div_ceil(2));
        Self {
            entries: [KvEntry::default(); N],
            len: 0,
        }
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
    pub fn life_fraction(&self) -> f32 {
        if N == 0 {
            0.0
        } else {
            1.0 - (self.len as f32 / N as f32)
        }
    }
    pub fn rollback(&mut self, new_len: usize) {
        self.len = new_len.min(N);
    }
    pub fn push(&mut self, key: &[f32; D], value: &[f32; D]) -> bool {
        if self.is_full() {
            return false;
        }
        let (k, ks) = quantize_to_i4::<D, P>(key);
        let (v, vs) = quantize_to_i4::<D, P>(value);
        self.entries[self.len] = KvEntry {
            key: k,
            value: v,
            key_scale: ks,
            value_scale: vs,
        };
        self.len += 1;
        true
    }
    pub fn attention_scores(&self, query: &[f32; D], scores: &mut [f32; N]) {
        let mut key = [0.0f32; D];
        let mut t = 0;
        while t < self.len {
            dequantize_i4::<D, P>(&self.entries[t].key, self.entries[t].key_scale, &mut key);
            let mut dot = 0.0;
            let mut i = 0;
            while i < D {
                dot += query[i] * key[i];
                i += 1;
            }
            scores[t] = dot / libm::sqrtf(D as f32);
            t += 1;
        }
    }
    pub fn weighted_values(&self, weights: &[f32; N], out: &mut [f32; D]) {
        let mut i = 0;
        while i < D {
            out[i] = 0.0;
            i += 1;
        }
        let mut value = [0.0f32; D];
        let mut t = 0;
        while t < self.len {
            dequantize_i4::<D, P>(
                &self.entries[t].value,
                self.entries[t].value_scale,
                &mut value,
            );
            let mut j = 0;
            while j < D {
                out[j] += weights[t] * value[j];
                j += 1;
            }
            t += 1;
        }
    }
}

impl<const D: usize, const P: usize, const N: usize> Default for Int4KvCache<D, P, N> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cache_life() {
        let mut c = Int4KvCache::<4, 2, 2>::new();
        assert_eq!(c.len(), 0);
        assert!(c.push(&[1.0; 4], &[2.0; 4]));
        assert!(c.life_fraction() < 1.0);
    }
}
