use crate::sample::argmax;

pub fn q8_deq(w: i8, scale: f32) -> f32 {
    w as f32 * scale
}

#[allow(clippy::too_many_arguments)]
pub fn rnn_step<const VOCAB: usize, const HIDDEN: usize>(
    ix: usize,
    wxh: &[i8],
    wxh_scale: f32,
    whh: &[i8],
    whh_scale: f32,
    why: &[i8],
    why_scale: f32,
    bh: &[i8],
    bh_scale: f32,
    by: &[i8],
    by_scale: f32,
    h: &mut [f32; HIDDEN],
    logits: &mut [f32; VOCAB],
) -> usize {
    let mut new_h = [0.0f32; HIDDEN];
    let mut i = 0;
    while i < HIDDEN {
        let mut sum = q8_deq(bh[i], bh_scale) + q8_deq(wxh[i * VOCAB + ix], wxh_scale);
        let mut j = 0;
        while j < HIDDEN {
            sum += q8_deq(whh[i * HIDDEN + j], whh_scale) * h[j];
            j += 1;
        }
        new_h[i] = libm::tanhf(sum);
        i += 1;
    }
    i = 0;
    while i < HIDDEN {
        h[i] = new_h[i];
        i += 1;
    }
    let mut v = 0;
    while v < VOCAB {
        let mut sum = q8_deq(by[v], by_scale);
        let mut j = 0;
        while j < HIDDEN {
            sum += q8_deq(why[v * HIDDEN + j], why_scale) * h[j];
            j += 1;
        }
        logits[v] = sum;
        v += 1;
    }
    argmax(logits)
}
