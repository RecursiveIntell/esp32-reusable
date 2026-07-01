#![no_std]

pub const MAGIC: [u8; 4] = *b"TEAI";

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SensorContext {
    pub temperature_c: Option<f32>,
    pub humidity_pct: Option<f32>,
    pub battery_v: Option<f32>,
    pub wifi_rssi: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeatureTransfer<const N: usize> {
    pub sentinel_prediction: f32,
    pub sentinel_confidence: f32,
    pub features: heapless::Vec<f32, N>,
    pub timestamp_ms: u64,
}

impl<const N: usize> FeatureTransfer<N> {
    pub fn new(prediction: f32, confidence: f32, timestamp_ms: u64) -> Self {
        Self {
            sentinel_prediction: prediction,
            sentinel_confidence: confidence,
            features: heapless::Vec::new(),
            timestamp_ms,
        }
    }
    pub fn should_wake(&self, threshold: f32) -> bool {
        self.sentinel_confidence < threshold
    }
    pub fn encode(&self, out: &mut [u8]) -> Option<usize> {
        let need = 4 + 4 + 4 + 4 + self.features.len() * 4 + 8;
        if out.len() < need {
            return None;
        }
        let mut p = 0;
        out[p..p + 4].copy_from_slice(&MAGIC);
        p += 4;
        out[p..p + 4].copy_from_slice(&self.sentinel_prediction.to_le_bytes());
        p += 4;
        out[p..p + 4].copy_from_slice(&self.sentinel_confidence.to_le_bytes());
        p += 4;
        out[p..p + 4].copy_from_slice(&(self.features.len() as u32).to_le_bytes());
        p += 4;
        for f in self.features.iter() {
            out[p..p + 4].copy_from_slice(&f.to_le_bytes());
            p += 4;
        }
        out[p..p + 8].copy_from_slice(&self.timestamp_ms.to_le_bytes());
        p += 8;
        Some(p)
    }
    pub fn decode(data: &[u8]) -> Option<Self> {
        if data.len() < 24 || data[0..4] != MAGIC {
            return None;
        }
        let mut p = 4;
        let pred = f32::from_le_bytes(data[p..p + 4].try_into().ok()?);
        p += 4;
        let conf = f32::from_le_bytes(data[p..p + 4].try_into().ok()?);
        p += 4;
        let len = u32::from_le_bytes(data[p..p + 4].try_into().ok()?) as usize;
        p += 4;
        if len > N || data.len() < 4 + 4 + 4 + 4 + len * 4 + 8 {
            return None;
        }
        let mut features = heapless::Vec::new();
        for _ in 0..len {
            features
                .push(f32::from_le_bytes(data[p..p + 4].try_into().ok()?))
                .ok()?;
            p += 4;
        }
        let ts = u64::from_le_bytes(data[p..p + 8].try_into().ok()?);
        Some(Self {
            sentinel_prediction: pred,
            sentinel_confidence: conf,
            features,
            timestamp_ms: ts,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip() {
        let mut f = FeatureTransfer::<4>::new(0.5, 0.7, 42);
        f.features.push(1.0).unwrap();
        let mut buf = [0u8; 64];
        let n = f.encode(&mut buf).unwrap();
        let d = FeatureTransfer::<4>::decode(&buf[..n]).unwrap();
        assert_eq!(d.features[0], 1.0);
        assert!(d.should_wake(0.75));
    }
}
