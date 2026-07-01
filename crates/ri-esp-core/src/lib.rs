#![no_std]

use heapless::String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadingStatus {
    Ok,
    SensorReadFailed,
    NotConnected,
    Stale,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvironmentReading {
    pub temperature_c: Option<f32>,
    pub humidity_pct: Option<f32>,
    pub heat_index_c: Option<f32>,
    pub status: ReadingStatus,
}

impl EnvironmentReading {
    pub const fn failed() -> Self {
        Self {
            temperature_c: None,
            humidity_pct: None,
            heat_index_c: None,
            status: ReadingStatus::SensorReadFailed,
        }
    }
    pub const fn new(temperature_c: f32, humidity_pct: f32, heat_index_c: Option<f32>) -> Self {
        Self {
            temperature_c: Some(temperature_c),
            humidity_pct: Some(humidity_pct),
            heat_index_c,
            status: ReadingStatus::Ok,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceStatus<const N: usize> {
    pub device_id: String<N>,
    pub uptime_ms: u64,
    pub wifi_rssi: Option<i32>,
    pub ip: String<32>,
}

pub trait Sensor {
    type Reading;
    type Error;
    fn read(&mut self) -> Result<Self::Reading, Self::Error>;
}

pub trait TextDisplay {
    type Error;
    fn clear(&mut self) -> Result<(), Self::Error>;
    fn write_line(&mut self, row: u8, text: &str) -> Result<(), Self::Error>;
}

pub trait StatusDisplay: TextDisplay {
    fn show_environment(&mut self, reading: &EnvironmentReading) -> Result<(), Self::Error> {
        self.clear()?;
        match reading.status {
            ReadingStatus::Ok => {
                self.write_line(0, "sensor ok")?;
                // Formatting floats is deliberately left to app/widget crates.
                self.write_line(1, "temp/humidity valid")
            }
            _ => self.write_line(0, "sensor failed"),
        }
    }
}

impl<T: TextDisplay> StatusDisplay for T {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn environment_reading_ok() {
        let r = EnvironmentReading::new(22.0, 44.0, Some(21.8));
        assert_eq!(r.status, ReadingStatus::Ok);
        assert_eq!(r.temperature_c, Some(22.0));
    }
}
