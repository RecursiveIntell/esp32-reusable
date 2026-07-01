#![no_std]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct I2cPins {
    pub sda: u8,
    pub scl: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ili9341Pins {
    pub mosi: u8,
    pub sck: u8,
    pub cs: u8,
    pub dc: u8,
    pub rst: u8,
    pub bl: u8,
}

pub mod original_esp32_devkit {
    use super::I2cPins;
    pub const DEFAULT_I2C: I2cPins = I2cPins { sda: 21, scl: 22 };
    pub const COMMON_DHT_PIN: u8 = 4;
}

pub mod esp32_2432s028 {
    use super::Ili9341Pins;
    pub const TFT: Ili9341Pins = Ili9341Pins {
        mosi: 13,
        sck: 14,
        cs: 15,
        dc: 2,
        rst: 4,
        bl: 21,
    };
    pub const TOUCH_CS: u8 = 5;
    pub const CHIP: &str = "esp32";
}

pub mod esp32s3_wroom1_n16r8 {
    use super::I2cPins;
    pub const SAFE_I2C: I2cPins = I2cPins { sda: 21, scl: 47 };
    pub const GPIO8_USED_BY_OCTAL_PSRAM: bool = true;
    pub const GPIO9_USED_BY_OCTAL_PSRAM: bool = true;
    pub fn is_known_bad_i2c_pin(pin: u8) -> bool {
        pin == 8 || pin == 9
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn s3_gpio8_9_blocked() {
        assert!(crate::esp32s3_wroom1_n16r8::is_known_bad_i2c_pin(8));
        assert!(crate::esp32s3_wroom1_n16r8::is_known_bad_i2c_pin(9));
        assert!(!crate::esp32s3_wroom1_n16r8::is_known_bad_i2c_pin(21));
    }
}
