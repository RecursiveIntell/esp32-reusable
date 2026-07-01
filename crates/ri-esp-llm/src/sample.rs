pub struct Lcg {
    state: u32,
}
impl Lcg {
    pub const fn new(seed: u32) -> Self {
        Self { state: seed }
    }
    pub fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        self.state
    }
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / 16777216.0
    }
}

pub fn argmax(logits: &[f32]) -> usize {
    let mut best = 0;
    let mut val = f32::NEG_INFINITY;
    for (i, &v) in logits.iter().enumerate() {
        if v > val {
            best = i;
            val = v;
        }
    }
    best
}

pub fn softmax<const N: usize>(scores: &[f32; N], len: usize, out: &mut [f32; N]) {
    if len == 0 {
        return;
    }
    let mut max = f32::NEG_INFINITY;
    let mut i = 0;
    while i < len {
        if scores[i] > max {
            max = scores[i];
        }
        i += 1;
    }
    let mut sum = 0.0;
    i = 0;
    while i < len {
        out[i] = libm::expf(scores[i] - max);
        sum += out[i];
        i += 1;
    }
    if sum > 0.0 {
        i = 0;
        while i < len {
            out[i] /= sum;
            i += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn argmax_works() {
        assert_eq!(argmax(&[0.1, 2.0, 1.0]), 1);
    }
}
