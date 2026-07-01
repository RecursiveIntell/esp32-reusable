#[inline]
pub fn pack_i4_pair(lo: i8, hi: i8) -> u8 {
    ((hi as u8) & 0x0f) << 4 | ((lo as u8) & 0x0f)
}

#[inline]
pub fn unpack_i4(packed: &[u8], index: usize) -> i8 {
    let n = if index.is_multiple_of(2) {
        packed[index / 2] & 0x0f
    } else {
        (packed[index / 2] >> 4) & 0x0f
    };
    if n & 0x08 != 0 {
        (n | 0xf0) as i8
    } else {
        n as i8
    }
}

pub fn quantize_to_i4<const N: usize, const P: usize>(src: &[f32; N]) -> ([u8; P], f32) {
    debug_assert!(P >= N.div_ceil(2));
    let mut max_abs = 0.0f32;
    let mut i = 0;
    while i < N {
        let a = if src[i] < 0.0 { -src[i] } else { src[i] };
        if a > max_abs {
            max_abs = a;
        }
        i += 1;
    }
    let scale = if max_abs > 0.0 { max_abs / 7.0 } else { 1.0 };
    let mut packed = [0u8; P];
    i = 0;
    while i < N {
        let scaled = src[i] / scale;
        let q: i8 = if scaled >= 7.0 {
            7
        } else if scaled <= -8.0 {
            -8
        } else {
            scaled as i8
        };
        if i.is_multiple_of(2) {
            packed[i / 2] = (q as u8) & 0x0f;
        } else {
            packed[i / 2] |= ((q as u8) & 0x0f) << 4;
        }
        i += 1;
    }
    (packed, scale)
}

pub fn dequantize_i4<const N: usize, const P: usize>(
    packed: &[u8; P],
    scale: f32,
    out: &mut [f32; N],
) {
    debug_assert!(P >= N.div_ceil(2));
    let mut i = 0;
    while i < N {
        out[i] = unpack_i4(packed, i) as f32 * scale;
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pack_unpack() {
        let p = [pack_i4_pair(-1, 3)];
        assert_eq!(unpack_i4(&p, 0), -1);
        assert_eq!(unpack_i4(&p, 1), 3);
    }
    #[test]
    fn quant_roundtrip_shape() {
        let x = [-1.0, 0.0, 1.0, 2.0];
        let (p, s) = quantize_to_i4::<4, 2>(&x);
        let mut out = [0.0; 4];
        dequantize_i4::<4, 2>(&p, s, &mut out);
        assert!(out[3] > out[2]);
    }
}
