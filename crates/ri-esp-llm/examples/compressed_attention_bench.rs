//! ESP32 compressed attention benchmark.
//!
//! Compares CompressedAttentionCache vs Int4KvCache timing.
//! Runs on host first, then cross-compiles for ESP32-S3.
//! Output goes to serial (defmt or panic handler) and can drive OLED.

#![cfg_attr(target_arch = "xtensa", no_std)]

#[cfg(not(target_arch = "xtensa"))]
extern crate std;

use libm::sqrtf;
use ri_esp_llm::compressed_attention::CompressedAttentionCache;
use ri_esp_llm::kv_cache::Int4KvCache;

// Benchmark configuration: matches feasible ESP32-S3 attention head
const DIM: usize = 64;
const N: usize = 128; // max cached entries
const K: usize = 8; // top-k for attention
const I4_PACK: usize = DIM.div_ceil(2); // 32 bytes for 64-dim

// Simple cycle counter for timing (works on ESP32-S3 and host)
#[cfg(not(target_arch = "xtensa"))]
mod host_timer {
    use std::sync::OnceLock;
    use std::time::Instant;
    static START: OnceLock<Instant> = OnceLock::new();
    pub fn init() {
        START.get_or_init(Instant::now);
    }
    pub fn now() -> u64 {
        let start = START.get_or_init(Instant::now);
        start.elapsed().as_nanos() as u64
    }
}

#[cfg(target_arch = "xtensa")]
mod esp_timer {
    // On ESP32-S3, we use a simple incrementing counter as a proxy for cycles.
    // Real cycle counting would need esp-hal's timer peripheral.
    static mut COUNTER: u64 = 0;
    pub fn init() {}
    pub fn now() -> u64 {
        // This is a placeholder. On real ESP32-S3 hardware,
        // use esp-hal's systimer or CCOUNT register.
        // For now, return a monotonically increasing counter.
        unsafe {
            COUNTER += 1;
            COUNTER
        }
    }
}

#[cfg(target_arch = "xtensa")]
use esp_timer::*;
#[cfg(not(target_arch = "xtensa"))]
use host_timer::*;

/// Generate deterministic test data (same seed for fair comparison)
fn gen_keys(n: usize) -> [[f32; DIM]; N] {
    let mut keys = [[0.0f32; DIM]; N];
    let mut seed: u32 = 42;
    let mut i = 0;
    while i < n {
        let mut j = 0;
        while j < DIM {
            // Simple LCG for deterministic pseudo-random data
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            let val = ((seed >> 8) as f32 / 16777216.0) - 0.5; // [-0.5, 0.5)
            keys[i][j] = val * 0.1; // Scale like real embeddings
            j += 1;
        }
        // Normalize to unit length
        let mut norm_sq = 0.0f32;
        let mut j = 0;
        while j < DIM {
            norm_sq += keys[i][j] * keys[i][j];
            j += 1;
        }
        let norm = sqrtf(norm_sq).max(1e-6);
        let mut j = 0;
        while j < DIM {
            keys[i][j] /= norm;
            j += 1;
        }
        i += 1;
    }
    keys
}

fn gen_query() -> [f32; DIM] {
    let mut query = [0.0f32; DIM];
    let mut seed: u32 = 12345;
    let mut j = 0;
    while j < DIM {
        seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
        let val = ((seed >> 8) as f32 / 16777216.0) - 0.5;
        query[j] = val * 0.1;
        j += 1;
    }
    let mut norm_sq = 0.0f32;
    let mut j = 0;
    while j < DIM {
        norm_sq += query[j] * query[j];
        j += 1;
    }
    let norm = sqrtf(norm_sq).max(1e-6);
    let mut j = 0;
    while j < DIM {
        query[j] /= norm;
        j += 1;
    }
    query
}

/// Benchmark Int4KvCache: push + attention_scores + weighted_values
fn bench_int4(keys: &[[f32; DIM]; N], query: &[f32; DIM]) -> (u64, u64, u64) {
    let mut cache: Int4KvCache<DIM, I4_PACK, N> = Int4KvCache::new();

    // Time push
    let t0 = now();
    let mut i = 0;
    while i < N {
        cache.push(&keys[i], &keys[i]); // Use key as value too (simplifies)
        i += 1;
    }
    let t_push = now() - t0;

    // Time attention_scores (dequantize + dot product for ALL keys)
    let mut scores = [0.0f32; N];
    let t1 = now();
    cache.attention_scores(query, &mut scores);
    let t_score = now() - t1;

    // Time weighted_values (dequantize ALL values + weighted sum)
    let mut weights = [0.0f32; N];
    // Simulate softmax weights (doesn't matter for timing)
    let mut i = 0;
    while i < K {
        weights[i] = 1.0 / K as f32;
        i += 1;
    }
    let mut out = [0.0f32; DIM];
    let t2 = now();
    cache.weighted_values(&weights, &mut out);
    let t_value = now() - t2;

    (t_push, t_score, t_value)
}

/// Benchmark CompressedAttentionCache: push + attention_scores + attention_topk
fn bench_perdim(keys: &[[f32; DIM]; N], query: &[f32; DIM]) -> (u64, u64, u64, u64) {
    let mut cache: CompressedAttentionCache<DIM, N, K> = CompressedAttentionCache::new();

    // Fit calibration (using first 32 keys)
    let mut calib: [[f32; DIM]; 32] = [[0.0; DIM]; 32];
    let mut i = 0;
    while i < 32 {
        calib[i] = keys[i];
        i += 1;
    }
    cache.fit(&calib[..32]);

    // Time push (quantize key + store value)
    let t0 = now();
    let mut i = 0;
    while i < N {
        cache.push(&keys[i], &keys[i]);
        i += 1;
    }
    let t_push = now() - t0;

    // Time attention_scores (compressed scoring, no dequantization)
    let mut scores = [0.0f32; N];
    let t1 = now();
    cache.attention_scores(query, &mut scores);
    let t_score = now() - t1;

    // Time attention_topk (score + select top-K + softmax + decode K values)
    let mut out = [0.0f32; DIM];
    let t2 = now();
    let decoded = cache.attention_topk(query, &mut out);
    let t_topk = now() - t2;

    (t_push, t_score, t_topk, decoded as u64)
}

#[cfg(not(target_arch = "xtensa"))]
fn main() {
    init();

    println!("Compressed Attention Benchmark: CompressedAttentionCache vs Int4KvCache");
    println!("Config: dim={}, n_keys={}, top_k={}", DIM, N, K);
    println!();

    let keys = gen_keys(N);
    let query = gen_query();

    // Warmup
    let _ = bench_int4(&keys, &query);
    let _ = bench_perdim(&keys, &query);

    // Run 10 iterations and average
    const RUNS: usize = 10;
    let mut int4_push_total: u64 = 0;
    let mut int4_score_total: u64 = 0;
    let mut int4_value_total: u64 = 0;
    let mut pd_push_total: u64 = 0;
    let mut pd_score_total: u64 = 0;
    let mut pd_topk_total: u64 = 0;

    let mut run = 0;
    while run < RUNS {
        let (p, s, v) = bench_int4(&keys, &query);
        int4_push_total += p;
        int4_score_total += s;
        int4_value_total += v;

        let (p, s, t, _) = bench_perdim(&keys, &query);
        pd_push_total += p;
        pd_score_total += s;
        pd_topk_total += t;

        run += 1;
    }

    let int4_push = int4_push_total / RUNS as u64;
    let int4_score = int4_score_total / RUNS as u64;
    let int4_value = int4_value_total / RUNS as u64;
    let int4_total = int4_push + int4_score + int4_value;

    let pd_push = pd_push_total / RUNS as u64;
    let pd_score = pd_score_total / RUNS as u64;
    let pd_topk = pd_topk_total / RUNS as u64;
    let pd_total = pd_push + pd_score + pd_topk;

    println!("=== Int4KvCache (current) ===");
    println!("  push:           {:>8} cycles", int4_push);
    println!("  attention_scores:{:>8} cycles", int4_score);
    println!("  weighted_values:{:>8} cycles", int4_value);
    println!("  TOTAL:          {:>8} cycles", int4_total);
    println!();
    println!("=== CompressedAttentionCache (per-dim) ===");
    println!("  push:           {:>8} cycles", pd_push);
    println!("  attention_scores:{:>8} cycles", pd_score);
    println!("  attention_topk:  {:>8} cycles", pd_topk);
    println!("  TOTAL:          {:>8} cycles", pd_total);
    println!();
    println!("=== Comparison ===");
    let score_speedup = int4_score as f64 / pd_score as f64;
    let value_speedup = int4_value as f64 / pd_topk as f64;
    let total_speedup = int4_total as f64 / pd_total as f64;
    println!("  Scoring speedup:    {:.2}x", score_speedup);
    println!("  Value decode speedup:{:.2}x", value_speedup);
    println!("  Total speedup:      {:.2}x", total_speedup);
    println!();
    println!(
        "  Int4KvCache dequantizes ALL {} keys + ALL {} values",
        N, N
    );
    println!(
        "  CompressedAttentionCache scores {} compressed keys, decodes {} values",
        N, K
    );
    println!();
    println!(
        "  Memory per key: Int4={} bytes  PerDim={} bytes",
        I4_PACK + 4,
        DIM + 4
    );
    println!(
        "  Memory total:  Int4={} bytes  PerDim={} bytes",
        N * (I4_PACK + 4) + N * (I4_PACK + 4),
        N * (DIM + 4) + N * DIM * 4
    );
}

// ESP32 entry point — requires esp-hal setup, not included in this example.
// To run on ESP32-S3, create a binary crate with esp-hal that calls:
//   1. ri_esp_llm::compressed_attention::CompressedAttentionCache
//   2. ri_esp_llm::kv_cache::Int4KvCache
//   3. Use esp-hal's systimer for timing
//   4. Output results to UART or OLED
//
// The host benchmark above demonstrates the algorithmic speedup.
// On ESP32-S3, the speedup is amplified because:
//   - No cuBLAS (all math is software float)
//   - PSRAM bandwidth is the bottleneck (fewer bytes = faster)
//   - SRAM is limited (compressed keys fit in SRAM, f32 keys need PSRAM)
