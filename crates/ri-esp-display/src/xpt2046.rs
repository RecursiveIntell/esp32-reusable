//! XPT2046 resistive touch controller driver for ESP32-2432S028 (CYD)
//!
//! Pin mapping for this board:
//!   MOSI = GPIO 32
//!   MISO = GPIO 39 (input-only pin!)
//!   SCK  = GPIO 25
//!   CS   = GPIO 33
//!   IRQ  = GPIO 36 (input-only pin, active low)
//!
//! Uses **software bit-bang SPI** to avoid hardware SPI peripheral
//! configuration complexity for the touch bus. The XPT2046 uses a
//! separate SPI bus from the ILI9341 TFT.
//!
//! Protocol: 12-bit ADC reading over SPI mode 0/1.
//!   - Send one command byte (MSB first)
//!   - Read two bytes back while clocking
//!   - Result is 12 bits; low 4 bits (or 3 bits) are don't-care.
//!
//! Maximum reliable SPI clock: 2.5 MHz.

use embedded_hal::digital::{InputPin, OutputPin};

/// XPT2046 analog channel selection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Xpt2046Channel {
    /// Y position measurement (differential mode) — channel 1
    YPosition = 0b101,
    /// X position measurement (differential mode) — channel 0
    XPosition = 0b001,
    /// Z1 pressure measurement
    Z1Pressure = 0b011,
    /// Z2 pressure measurement
    Z2Pressure = 0b010,
}

/// Build the 8-bit command byte for a given channel.
///
/// Bit layout:
///   [7]   = S (start) — always 1
///   [6:4] = A2:A0 — analog channel select
///   [3]   = MODE — 0 = 12-bit, 1 = 8-bit
///   [2]   = SER/DFR — 0 = differential (recommended for X/Y)
///   [1:0] = PD1:PD0 — power-down mode (11 = always powered)
fn command_byte(channel: Xpt2046Channel, eight_bit: bool, power_down: u8) -> u8 {
    0x80 | ((channel as u8) << 4) | ((eight_bit as u8) << 3) | (power_down & 0x03)
}

/// 12-bit raw touch point (0..4095).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawTouchPoint {
    pub x: u16,
    pub y: u16,
    pub pressure: u16,
}

/// Minimum portable touch driver: bit-bang SPI on two output pins,
/// one input pin for MISO, one output chip-select, one input IRQ.
#[derive(Debug)]
pub struct TouchBitBang<MOSI, MISO, SCK, CS, IRQ> {
    mosi: MOSI,
    miso: MISO,
    sck: SCK,
    cs: CS,
    irq: IRQ,
}

impl<MOSI, MISO, SCK, CS, IRQ, E> TouchBitBang<MOSI, MISO, SCK, CS, IRQ>
where
    MOSI: OutputPin<Error = E>,
    MISO: InputPin<Error = E>,
    SCK: OutputPin<Error = E>,
    CS: OutputPin<Error = E>,
    IRQ: InputPin<Error = E>,
{
    /// Create a new XPT2046 bit-bang driver.
    ///
    /// Pins are in idle state: SCK low, CS high.
    pub fn new(mosi: MOSI, miso: MISO, sck: SCK, cs: CS, irq: IRQ) -> Result<Self, E> {
        let mut driver = Self {
            mosi,
            miso,
            sck,
            cs,
            irq,
        };
        driver.sck.set_low()?;
        driver.cs.set_high()?;
        Ok(driver)
    }

    /// Minimal SPI clock half-period delay.
    /// At 240 MHz, 3 spin_loop calls ≈ 12-15 ns, well within
    /// XPT2046's ~400 ns minimum half-period (2.5 MHz max).
    #[inline(always)]
    fn half_cycle_delay() {
        core::hint::spin_loop();
        core::hint::spin_loop();
        core::hint::spin_loop();
    }

    /// Bit-bang write a byte to the XPT2046 while simultaneously
    /// reading a byte from MISO. SPI mode 0: clock idle low, data
    /// sampled on rising edge, shifted out on falling edge.
    fn transfer_byte(&mut self, tx: u8) -> Result<u8, E> {
        let mut rx = 0u8;
        for bit in (0..8).rev() {
            // Write MOSI (data changes on falling edge)
            if tx & (1 << bit) != 0 {
                self.mosi.set_high()?;
            } else {
                self.mosi.set_low()?;
            }
            Self::half_cycle_delay();

            // Rising edge — XPT2046 shifts data out; sample MISO
            self.sck.set_high()?;
            Self::half_cycle_delay();
            if self.miso.is_high()? {
                rx |= 1 << bit;
            }

            // Falling edge
            self.sck.set_low()?;
            Self::half_cycle_delay();
        }
        Ok(rx)
    }

    /// Perform a full SPI transfer with chip-select asserted.
    /// `tx_data` is written byte-by-byte. The returned buffer
    /// contains the received bytes for each transmitted byte.
    fn transfer(&mut self, tx_data: &[u8]) -> Result<heapless::Vec<u8, 8>, E> {
        self.cs.set_low()?;
        let mut rx_buf: heapless::Vec<u8, 8> = heapless::Vec::new();
        for &byte in tx_data {
            let rx = self.transfer_byte(byte)?;
            let _ = rx_buf.push(rx);
        }
        self.cs.set_high()?;
        Self::half_cycle_delay();
        Ok(rx_buf)
    }

    /// Read a raw 12-bit ADC value from the given channel.
    /// Returns a value in range 0..4095.
    pub fn read_channel(&mut self, channel: Xpt2046Channel) -> Result<u16, E> {
        let cmd = command_byte(channel, false, 0b11); // 12-bit, differential, always powered
        let rx = self.transfer(&[cmd, 0x00, 0x00])?;
        // The XPT2046 returns the 12-bit value spanning byte 1 and byte 2:
        //   byte1: bits 11-4 of ADC result
        //   byte2: bits 3-0 of ADC result (top nibble), low nibble undefined
        let high = rx[1] as u16;
        let low = rx[2] as u16;
        Ok((high << 4) | (low >> 4))
    }

    /// Check if the screen is currently being touched.
    /// IRQ pin goes low when a touch is detected (active low).
    pub fn is_pressed(&mut self) -> Result<bool, E> {
        // IRQ is active low: low means pressed
        Ok(self.irq.is_low()?)
    }

    /// Read raw X coordinate (12-bit).
    pub fn read_x_raw(&mut self) -> Result<u16, E> {
        self.read_channel(Xpt2046Channel::XPosition)
    }

    /// Read raw Y coordinate (12-bit).
    pub fn read_y_raw(&mut self) -> Result<u16, E> {
        self.read_channel(Xpt2046Channel::YPosition)
    }

    /// Read approximate pressure.
    /// Higher values = more pressure. 0 = no touch.
    pub fn read_pressure(&mut self) -> Result<u16, E> {
        let z1 = self.read_channel(Xpt2046Channel::Z1Pressure)?;
        let z2 = self.read_channel(Xpt2046Channel::Z2Pressure)?;
        if z2 == 0 || z2 >= z1 {
            return Ok(0);
        }
        // Z2/Z1 ratio gives an approximate pressure indication
        let p = ((z1 as u32) * 4096) / (z1 as u32 - z2 as u32);
        Ok(p.min(4095) as u16)
    }

    /// Read both X and Y in a single transaction (two channel reads).
    pub fn read_xy_raw(&mut self) -> Result<RawTouchPoint, E> {
        let x = self.read_x_raw()?;
        let y = self.read_y_raw()?;
        let pressure = self.read_pressure()?;
        Ok(RawTouchPoint { x, y, pressure })
    }

    /// Release the pins (consumes the driver, returns pins).
    pub fn release(self) -> (MOSI, MISO, SCK, CS, IRQ) {
        (self.mosi, self.miso, self.sck, self.cs, self.irq)
    }
}

/// Calibration data for mapping touch coordinates to screen pixels.
///
/// The ESP32-2432S028 screen is 320x240 pixels. The XPT2046 returns
/// 12-bit values (0-4095). Calibration maps raw ADC ranges to pixel
/// ranges, with optional axis swap and inversion for different panel
/// variants.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TouchCalibration {
    /// Minimum raw X value (left edge of screen)
    pub x_min: u16,
    /// Maximum raw X value (right edge of screen)
    pub x_max: u16,
    /// Minimum raw Y value (top edge of screen)
    pub y_min: u16,
    /// Maximum raw Y value (bottom edge of screen)
    pub y_max: u16,
    /// Swap X and Y axes before mapping
    pub swap_xy: bool,
    /// Invert X axis (mirror horizontally)
    pub invert_x: bool,
    /// Invert Y axis (mirror vertically)
    pub invert_y: bool,
}

impl TouchCalibration {
    /// Create a new calibration.
    pub const fn new(
        x_min: u16,
        x_max: u16,
        y_min: u16,
        y_max: u16,
        swap_xy: bool,
        invert_x: bool,
        invert_y: bool,
    ) -> Self {
        Self {
            x_min,
            x_max,
            y_min,
            y_max,
            swap_xy,
            invert_x,
            invert_y,
        }
    }

    /// Default approximate calibration for the ESP32-2432S028.
    ///
    /// These values were sampled from a typical CYD and should work
    /// within ±10 pixels. Run the calibration procedure for precise
    /// mapping.
    ///
    /// The CYD panel typically has:
    ///   - X: ~200..3800  (left to right in landscape)
    ///   - Y: ~300..3700  (top to bottom in landscape)
    ///   - No swap, no inversion needed
    pub const fn default_cyd() -> Self {
        Self {
            x_min: 200,
            x_max: 3800,
            y_min: 300,
            y_max: 3700,
            swap_xy: false,
            invert_x: false,
            invert_y: false,
        }
    }

    /// Returns true if the calibration range is plausible (min < max).
    pub fn is_valid(&self) -> bool {
        self.x_min < self.x_max && self.y_min < self.y_max
    }

    /// Map a raw (x, y) touch coordinate to screen pixel coordinates.
    ///
    /// Screen is 320 pixels wide, 240 pixels tall.
    /// Returns (screen_x, screen_y) in 0..319, 0..239.
    pub fn map(&self, raw_x: u16, raw_y: u16) -> (u16, u16) {
        let (mut rx, mut ry) = if self.swap_xy {
            (raw_y, raw_x)
        } else {
            (raw_x, raw_y)
        };

        // Clamp to calibration range to avoid divide-by-zero panic
        if rx < self.x_min {
            rx = self.x_min;
        }
        if rx > self.x_max {
            rx = self.x_max;
        }
        if ry < self.y_min {
            ry = self.y_min;
        }
        if ry > self.y_max {
            ry = self.y_max;
        }

        let range_x = self.x_max - self.x_min;
        let range_y = self.y_max - self.y_min;

        let mut sx = if range_x > 0 {
            ((rx - self.x_min) as u32 * 319) / range_x as u32
        } else {
            0
        } as u16;

        let mut sy = if range_y > 0 {
            ((ry - self.y_min) as u32 * 239) / range_y as u32
        } else {
            0
        } as u16;

        if self.invert_x {
            sx = 319 - sx;
        }
        if self.invert_y {
            sy = 239 - sy;
        }

        (sx, sy)
    }
}

/// Touch event type for the polling state machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchEvent {
    /// Screen was tapped (touch-down then touch-up quickly, same area)
    Tap { x: u16, y: u16 },
    /// Finger is dragging across the screen
    Drag { x: u16, y: u16 },
    /// Finger was lifted
    Release { x: u16, y: u16 },
}

/// Simple touch state machine for polling-based touch handling.
///
/// State machine:
///   Idle → (IRQ low) → Pressed → (moved > threshold) → Dragging
///                               → (IRQ high) → Release → Idle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TouchState {
    Idle,
    Debounce {
        count: u8,
        x: u16,
        y: u16,
    },
    Pressed {
        x: u16,
        y: u16,
        start_x: u16,
        start_y: u16,
    },
    Dragging {
        x: u16,
        y: u16,
    },
}

/// Polling touch controller that drives the state machine.
pub struct TouchController<MOSI, MISO, SCK, CS, IRQ> {
    driver: TouchBitBang<MOSI, MISO, SCK, CS, IRQ>,
    calibration: TouchCalibration,
    state: TouchState,
    drag_threshold: u16,
}

impl<MOSI, MISO, SCK, CS, IRQ, E> TouchController<MOSI, MISO, SCK, CS, IRQ>
where
    MOSI: OutputPin<Error = E>,
    MISO: InputPin<Error = E>,
    SCK: OutputPin<Error = E>,
    CS: OutputPin<Error = E>,
    IRQ: InputPin<Error = E>,
{
    /// Create a new touch controller with a calibration.
    pub fn new(
        driver: TouchBitBang<MOSI, MISO, SCK, CS, IRQ>,
        calibration: TouchCalibration,
    ) -> Self {
        Self {
            driver,
            calibration,
            state: TouchState::Idle,
            drag_threshold: 8, // 8 pixels before we call it a drag
        }
    }

    /// Set the drag threshold in pixels (default: 8).
    pub fn set_drag_threshold(&mut self, pixels: u16) {
        self.drag_threshold = pixels;
    }

    /// Get the current calibration (for display or adjustment).
    pub fn calibration(&self) -> &TouchCalibration {
        &self.calibration
    }

    /// Get mutable calibration reference (for runtime calibration).
    pub fn calibration_mut(&mut self) -> &mut TouchCalibration {
        &mut self.calibration
    }

    /// Poll the touch controller. Should be called at ~30-60 Hz from
    /// the main loop.
    ///
    /// Returns `None` if no event occurred this poll cycle.
    /// Returns `Some(TouchEvent)` when a touch, drag, or release
    /// event is detected.
    pub fn poll(&mut self) -> Result<Option<TouchEvent>, E> {
        // Take state out to avoid borrow conflict with &mut self
        let current_state = core::mem::replace(&mut self.state, TouchState::Idle);

        let (next_state, event) = match current_state {
            TouchState::Idle => {
                if self.driver.is_pressed()? {
                    (
                        TouchState::Debounce {
                            count: 0,
                            x: 0,
                            y: 0,
                        },
                        None,
                    )
                } else {
                    (TouchState::Idle, None)
                }
            }

            TouchState::Debounce { count, x, y } => {
                if !self.driver.is_pressed()? {
                    // False trigger — back to idle
                    (TouchState::Idle, None)
                } else {
                    // Sample for debounce
                    let raw = self.driver.read_xy_raw()?;
                    let (sx, sy) = self.calibration.map(raw.x, raw.y);
                    let new_count = count + 1;
                    if new_count >= 3 {
                        // Debounced — finger is pressed
                        let state = TouchState::Pressed {
                            x: sx,
                            y: sy,
                            start_x: sx,
                            start_y: sy,
                        };
                        (state, None)
                    } else {
                        let state = TouchState::Debounce {
                            count: new_count,
                            x: sx,
                            y: sy,
                        };
                        (state, None)
                    }
                }
            }

            TouchState::Pressed {
                x,
                y,
                start_x,
                start_y,
            } => {
                if !self.driver.is_pressed()? {
                    // Finger lifted — it was a tap
                    (TouchState::Idle, Some(TouchEvent::Tap { x, y }))
                } else {
                    // Sample new position
                    let raw = self.driver.read_xy_raw()?;
                    let (sx, sy) = self.calibration.map(raw.x, raw.y);

                    // Check if moved enough to be a drag
                    let dx = if sx > start_x {
                        sx - start_x
                    } else {
                        start_x - sx
                    };
                    let dy = if sy > start_y {
                        sy - start_y
                    } else {
                        start_y - sy
                    };

                    if dx > self.drag_threshold || dy > self.drag_threshold {
                        let state = TouchState::Dragging { x: sx, y: sy };
                        (state, Some(TouchEvent::Drag { x: sx, y: sy }))
                    } else {
                        let state = TouchState::Pressed {
                            x: sx,
                            y: sy,
                            start_x,
                            start_y,
                        };
                        (state, None)
                    }
                }
            }

            TouchState::Dragging { x, y } => {
                if !self.driver.is_pressed()? {
                    // Finger lifted during drag
                    (TouchState::Idle, Some(TouchEvent::Release { x, y }))
                } else {
                    // Sample and report drag
                    let raw = self.driver.read_xy_raw()?;
                    let (sx, sy) = self.calibration.map(raw.x, raw.y);
                    let state = TouchState::Dragging { x: sx, y: sy };
                    (state, Some(TouchEvent::Drag { x: sx, y: sy }))
                }
            }
        };

        self.state = next_state;
        Ok(event)
    }

    /// Consume the controller and release the underlying pins.
    pub fn release(self) -> TouchBitBang<MOSI, MISO, SCK, CS, IRQ> {
        self.driver
    }
}
