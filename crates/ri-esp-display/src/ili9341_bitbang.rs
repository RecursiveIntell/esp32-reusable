use embedded_hal::digital::OutputPin;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb565(pub u16);
impl Rgb565 {
    pub const BLACK: Self = Self(0x0000);
    pub const WHITE: Self = Self(0xffff);
    pub const RED: Self = Self(0xf800);
    pub const GREEN: Self = Self(0x07e0);
    pub const BLUE: Self = Self(0x001f);
    pub const YELLOW: Self = Self(0xffe0);
    pub const CYAN: Self = Self(0x07ff);
}

pub const WIDTH: u16 = 320;
pub const HEIGHT: u16 = 240;

pub struct BitBangIli9341<MOSI, SCK, CS, DC, RST> {
    mosi: MOSI,
    sck: SCK,
    cs: CS,
    dc: DC,
    rst: RST,
}

impl<MOSI, SCK, CS, DC, RST, E> BitBangIli9341<MOSI, SCK, CS, DC, RST>
where
    MOSI: OutputPin<Error = E>,
    SCK: OutputPin<Error = E>,
    CS: OutputPin<Error = E>,
    DC: OutputPin<Error = E>,
    RST: OutputPin<Error = E>,
{
    pub fn new(
        mut mosi: MOSI,
        mut sck: SCK,
        mut cs: CS,
        mut dc: DC,
        mut rst: RST,
    ) -> Result<Self, E> {
        sck.set_low()?;
        cs.set_high()?;
        dc.set_low()?;
        rst.set_high()?;
        mosi.set_low()?;
        Ok(Self {
            mosi,
            sck,
            cs,
            dc,
            rst,
        })
    }
    #[inline(always)]
    fn bit_delay() {
        core::hint::spin_loop();
        core::hint::spin_loop();
        core::hint::spin_loop();
    }
    fn write_byte(&mut self, byte: u8) -> Result<(), E> {
        for bit in (0..8).rev() {
            if byte & (1 << bit) != 0 {
                self.mosi.set_high()?;
            } else {
                self.mosi.set_low()?;
            }
            self.sck.set_high()?;
            Self::bit_delay();
            self.sck.set_low()?;
            Self::bit_delay();
        }
        Ok(())
    }
    fn write(&mut self, data: &[u8]) -> Result<(), E> {
        for &b in data {
            self.write_byte(b)?;
        }
        Ok(())
    }
    pub fn command(&mut self, cmd: u8) -> Result<(), E> {
        self.cs.set_low()?;
        self.dc.set_low()?;
        self.write_byte(cmd)?;
        self.cs.set_high()
    }
    pub fn data(&mut self, data: &[u8]) -> Result<(), E> {
        self.cs.set_low()?;
        self.dc.set_high()?;
        self.write(data)?;
        self.cs.set_high()
    }
    pub fn command_data(&mut self, cmd: u8, data: &[u8]) -> Result<(), E> {
        self.command(cmd)?;
        self.data(data)
    }
    pub fn reset_pulse_blocking(&mut self) -> Result<(), E> {
        self.rst.set_low()?;
        for _ in 0..2_400_000 {
            core::hint::spin_loop();
        }
        self.rst.set_high()
    }
    pub fn init_no_delay(&mut self) -> Result<(), E> {
        self.command(0x01)?; // SWRESET
        self.command(0x11)?; // SLPOUT
        self.command_data(0x3a, &[0x55])?; // RGB565
        self.command_data(0x36, &[0x20])?; // landscape used by ESP32-2432S028 session
        self.command(0x29) // DISPON
    }
    pub fn set_window(&mut self, x0: u16, y0: u16, x1: u16, y1: u16) -> Result<(), E> {
        self.command_data(
            0x2a,
            &[(x0 >> 8) as u8, x0 as u8, (x1 >> 8) as u8, x1 as u8],
        )?;
        self.command_data(
            0x2b,
            &[(y0 >> 8) as u8, y0 as u8, (y1 >> 8) as u8, y1 as u8],
        )?;
        self.command(0x2c)
    }
    pub fn fill_rect(
        &mut self,
        x0: u16,
        y0: u16,
        x1: u16,
        y1: u16,
        color: Rgb565,
    ) -> Result<(), E> {
        self.set_window(x0, y0, x1, y1)?;
        self.cs.set_low()?;
        self.dc.set_high()?;
        let hi = (color.0 >> 8) as u8;
        let lo = color.0 as u8;
        let count = (x1 - x0 + 1) as usize * (y1 - y0 + 1) as usize;
        for _ in 0..count {
            self.write(&[hi, lo])?;
        }
        self.cs.set_high()
    }
    pub fn fill_screen(&mut self, color: Rgb565) -> Result<(), E> {
        self.fill_rect(0, 0, WIDTH - 1, HEIGHT - 1, color)
    }
}
