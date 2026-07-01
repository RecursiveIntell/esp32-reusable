use embedded_hal::i2c::I2c;

const LCD_EN: u8 = 0b0100_0000;
const LCD_RS: u8 = 0b0001_0000;
const LCD_BL: u8 = 0b1000_0000;

pub struct Lcd20x4<I2C> {
    i2c: I2C,
    addr: u8,
    backlight: bool,
}

impl<I2C, E> Lcd20x4<I2C>
where
    I2C: I2c<Error = E>,
{
    pub fn new(i2c: I2C, addr: u8) -> Self {
        Self {
            i2c,
            addr,
            backlight: true,
        }
    }
    pub fn release(self) -> I2C {
        self.i2c
    }
    fn write_byte(&mut self, data: u8) -> Result<(), E> {
        self.i2c.write(self.addr, &[data])
    }
    fn send_nibble(&mut self, nibble: u8, rs: bool) -> Result<(), E> {
        let rs_bit = if rs { LCD_RS } else { 0 };
        let bl_bit = if self.backlight { LCD_BL } else { 0 };
        let data = (nibble & 0x0f) | rs_bit | bl_bit;
        self.write_byte(data | LCD_EN)?;
        for _ in 0..240 {
            core::hint::spin_loop();
        }
        self.write_byte(data & !LCD_EN)
    }
    fn send_byte(&mut self, byte: u8, rs: bool) -> Result<(), E> {
        self.send_nibble(byte >> 4, rs)?;
        self.send_nibble(byte & 0x0f, rs)
    }
    fn command(&mut self, cmd: u8) -> Result<(), E> {
        self.send_byte(cmd, false)
    }
    fn write_char(&mut self, c: char) -> Result<(), E> {
        self.send_byte(c as u8, true)
    }
    pub fn init(&mut self) -> Result<(), E> {
        self.send_nibble(0x03, false)?;
        for _ in 0..10000 {
            core::hint::spin_loop();
        }
        self.send_nibble(0x03, false)?;
        for _ in 0..10000 {
            core::hint::spin_loop();
        }
        self.send_nibble(0x03, false)?;
        for _ in 0..10000 {
            core::hint::spin_loop();
        }
        self.send_nibble(0x02, false)?;
        for _ in 0..10000 {
            core::hint::spin_loop();
        }
        self.command(0x28)?;
        self.command(0x08)?;
        self.clear()?;
        self.command(0x06)?;
        self.command(0x0c)
    }
    pub fn clear(&mut self) -> Result<(), E> {
        self.command(0x01)
    }
    pub fn set_cursor(&mut self, row: u8, col: u8) -> Result<(), E> {
        let base = match row {
            0 => 0x00,
            1 => 0x40,
            2 => 0x14,
            3 => 0x54,
            _ => 0x00,
        };
        self.command(0x80 | (base + col))
    }
    pub fn write_str(&mut self, s: &str) -> Result<(), E> {
        for c in s.chars() {
            self.write_char(c)?;
        }
        Ok(())
    }
    pub fn write_at(&mut self, row: u8, col: u8, s: &str) -> Result<(), E> {
        self.set_cursor(row, col)?;
        self.write_str(s)
    }
    pub fn set_backlight(&mut self, on: bool) {
        self.backlight = on;
    }
}
