//! A Driver for the Waveshare 1.54" v3 E-Ink Display
//!
//! GDEW0154M09

use embedded_hal::{delay::*, digital::*, spi::SpiDevice};

use crate::color::Color;
use crate::interface::DisplayInterface;
use crate::traits::{InternalWiAdditions, RefreshLut, WaveshareDisplay};

pub(crate) mod command;
use self::command::Command;

/// Width of the display
pub const WIDTH: u32 = 200;
/// Height of the display
pub const HEIGHT: u32 = 200;
/// Default Background Color
pub const DEFAULT_BACKGROUND_COLOR: Color = Color::White;
const IS_BUSY_LOW: bool = true;
const SINGLE_BYTE_WRITE: bool = true;

#[cfg(feature = "graphics")]
pub use crate::epd1in54::Display1in54;

/// Epd1in54 v3 driver
pub struct Epd1in54<SPI, BUSY, DC, RST, DELAY> {
    /// Connection Interface
    interface: DisplayInterface<SPI, BUSY, DC, RST, DELAY, SINGLE_BYTE_WRITE>,
    /// Background Color
    color: Color,
}

impl<SPI, BUSY, DC, RST, DELAY> InternalWiAdditions<SPI, BUSY, DC, RST, DELAY>
    for Epd1in54<SPI, BUSY, DC, RST, DELAY>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    fn init(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        // Reset the device
        self.interface.reset(delay, 10_000, 10_000);

        // Panel Setting
        self.interface
            .cmd_with_data(spi, Command::PanelSetting, &[0xDf, 0x0e])?;

        // Internal codes (Magic numbers from Arduino driver)
        self.interface
            .cmd_with_data(spi, Command::Internal4D, &[0x55])?;
        self.interface
            .cmd_with_data(spi, Command::InternalAA, &[0x0f])?;
        self.interface
            .cmd_with_data(spi, Command::InternalE9, &[0x02])?;
        self.interface
            .cmd_with_data(spi, Command::InternalB6, &[0x11])?;
        self.interface
            .cmd_with_data(spi, Command::InternalF3, &[0x0a])?;

        // Resolution setting
        self.interface
            .cmd_with_data(spi, Command::ResolutionSetting, &[0xc8, 0x00, 0xc8])?;

        // Tcon setting
        self.interface
            .cmd_with_data(spi, Command::TconSetting, &[0x00])?;

        // VCOM
        self.interface
            .cmd_with_data(spi, Command::VcomAndDataIntervalSetting, &[0x97])?;

        // Internal code
        self.interface
            .cmd_with_data(spi, Command::InternalE3, &[0x00])?;

        // Power on
        self.interface.cmd(spi, Command::PowerOn)?;
        delay.delay_ms(100);
        self.wait_until_idle(spi, delay)?;

        Ok(())
    }
}

impl<SPI, BUSY, DC, RST, DELAY> WaveshareDisplay<SPI, BUSY, DC, RST, DELAY>
    for Epd1in54<SPI, BUSY, DC, RST, DELAY>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    type DisplayColor = Color;
    fn new(
        spi: &mut SPI,
        busy: BUSY,
        dc: DC,
        rst: RST,
        delay: &mut DELAY,
        delay_us: Option<u32>,
    ) -> Result<Self, SPI::Error> {
        let interface = DisplayInterface::new(busy, dc, rst, delay_us);
        let color = DEFAULT_BACKGROUND_COLOR;

        let mut epd = Epd1in54 { interface, color };

        epd.init(spi, delay)?;

        Ok(epd)
    }

    fn sleep(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.interface.cmd(spi, Command::PowerOff)?;
        self.wait_until_idle(spi, delay)?;
        delay.delay_ms(1000); // Wait for discharge?
        self.interface
            .cmd_with_data(spi, Command::DeepSleep, &[0xA5])?;
        Ok(())
    }

    fn wake_up(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.init(spi, delay)
    }

    fn set_background_color(&mut self, color: Color) {
        self.color = color;
    }

    fn background_color(&self) -> &Color {
        &self.color
    }

    fn width(&self) -> u32 {
        WIDTH
    }

    fn height(&self) -> u32 {
        HEIGHT
    }

    fn update_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        self.use_full_frame(spi, delay)?;
        self.wait_until_idle(spi, delay)?;

        // Based on Arduino:
        // 0x10 -> Old Data (0xFF/White for "Clear" to "Image", or "OldImage" for "Image" to "Image")
        // Since we don't track old frame, we write background color (Old state assumption)
        self.interface.cmd(spi, Command::DataStartTransmission1)?;
        self.interface
            .data_x_times(spi, self.color.get_byte_value(), WIDTH / 8 * HEIGHT)?;

        // 0x13 -> New Data
        self.interface
            .cmd_with_data(spi, Command::DataStartTransmission2, buffer)?;

        Ok(())
    }

    fn update_partial_frame(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        buffer: &[u8],
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<(), SPI::Error> {
        self.wait_until_idle(spi, delay)?;
        self.set_ram_area(spi, delay, x, y, x + width, y + height)?;
        self.set_ram_counter(spi, delay, x, y)?;

        self.interface.cmd(spi, Command::DataStartTransmission1)?;
        self.interface
            .data_x_times(spi, self.color.get_byte_value(), width / 8 * height)?;

        self.interface
            .cmd_with_data(spi, Command::DataStartTransmission2, buffer)?;
        Ok(())
    }

    fn display_frame(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.wait_until_idle(spi, delay)?;
        self.interface.cmd(spi, Command::DisplayRefresh)?;
        // The delay is necessary, 200uS at least!!!
        delay.delay_ms(10);
        self.wait_until_idle(spi, delay)?;
        Ok(())
    }

    fn update_and_display_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        self.update_frame(spi, buffer, delay)?;
        self.display_frame(spi, delay)?;
        Ok(())
    }

    fn clear_frame(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.wait_until_idle(spi, delay)?;
        self.use_full_frame(spi, delay)?;

        // Clear is sending 0x00 to 0x10 and 0xFF to 0x13 in Arduino 'PIC_display_Clean' ?
        // Wait, PIC_display_Clean: 0x10 -> 0x00. 0x13 -> 0xFF.
        // 0x00 is Black? 0xFF is White.
        // So sending 0x10(Black) and 0x13(White).
        // This effectively drives Black to White?

        // If we want to clear to Background Color (usually white).
        let color_value = self.color.get_byte_value();

        // However, standard Clear often implies removing everything.
        // Lets follow Arduino Clean procedure but using our Background color for New Data

        self.interface.cmd(spi, Command::DataStartTransmission1)?;
        self.interface.data_x_times(spi, 0x00, WIDTH / 8 * HEIGHT)?; // 0x00 in 0x10 (from Arduino)

        self.interface.cmd(spi, Command::DataStartTransmission2)?;
        // New data is 0xFF (White) typically. Or self.color.
        self.interface
            .data_x_times(spi, color_value, WIDTH / 8 * HEIGHT)?;

        self.display_frame(spi, delay)?;

        Ok(())
    }

    fn set_lut(
        &mut self,
        _spi: &mut SPI,
        _delay: &mut DELAY,
        _refresh_rate: Option<RefreshLut>,
    ) -> Result<(), SPI::Error> {
        // GDEW0154M09 uses internal LUTs and doesn't seem to support custom LUT downloads via SPI
        // in the example code.
        Ok(())
    }

    fn wait_until_idle(&mut self, _spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.interface.wait_until_idle(delay, IS_BUSY_LOW);
        Ok(())
    }
}

impl<SPI, BUSY, DC, RST, DELAY> Epd1in54<SPI, BUSY, DC, RST, DELAY>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    fn use_full_frame(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        // choose full frame/ram
        self.set_ram_area(spi, delay, 0, 0, WIDTH - 1, HEIGHT - 1)?;

        // start from the beginning
        self.set_ram_counter(spi, delay, 0, 0)
    }

    fn set_ram_area(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        start_x: u32,
        start_y: u32,
        end_x: u32,
        end_y: u32,
    ) -> Result<(), SPI::Error> {
        self.wait_until_idle(spi, delay)?;
        assert!(start_x < end_x);
        assert!(start_y < end_y);

        // x is positioned in bytes, so the last 3 bits which show the position inside a byte in the ram
        // aren't relevant
        self.interface.cmd_with_data(
            spi,
            Command::SetRamXAddressStartEndPosition,
            &[(start_x >> 3) as u8, (end_x >> 3) as u8],
        )?;

        // 2 Databytes: A[7:0] & 0..A[8] for each - start and end
        self.interface.cmd_with_data(
            spi,
            Command::SetRamYAddressStartEndPosition,
            &[
                start_y as u8,
                (start_y >> 8) as u8,
                end_y as u8,
                (end_y >> 8) as u8,
            ],
        )?;
        Ok(())
    }

    fn set_ram_counter(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        x: u32,
        y: u32,
    ) -> Result<(), SPI::Error> {
        self.wait_until_idle(spi, delay)?;
        // x is positioned in bytes, so the last 3 bits which show the position inside a byte in the ram
        // aren't relevant
        self.interface
            .cmd_with_data(spi, Command::SetRamXAddressCounter, &[(x >> 3) as u8])?;

        // 2 Databytes: A[7:0] & 0..A[8]
        self.interface.cmd_with_data(
            spi,
            Command::SetRamYAddressCounter,
            &[y as u8, (y >> 8) as u8],
        )?;
        Ok(())
    }
}
