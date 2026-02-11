//! SPI Commands for the Waveshare 1.54" v3 E-Ink Display

use crate::traits;

#[allow(dead_code)]
#[derive(Copy, Clone)]
pub(crate) enum Command {
    PanelSetting = 0x00,

    PowerOff = 0x02,
    PowerOn = 0x04,
    DeepSleep = 0x07,
    DataStartTransmission1 = 0x10,
    DisplayRefresh = 0x12,
    DataStartTransmission2 = 0x13,

    Internal4D = 0x4D,
    VcomAndDataIntervalSetting = 0x50,
    TconSetting = 0x60,
    ResolutionSetting = 0x61,

    InternalAA = 0xAA,
    InternalB6 = 0xB6,
    InternalE3 = 0xE3,
    InternalE9 = 0xE9,
    InternalF3 = 0xF3,

    SetRamXAddressStartEndPosition = 0x44,
    SetRamYAddressStartEndPosition = 0x45,
    SetRamXAddressCounter = 0x4E,
    SetRamYAddressCounter = 0x4F,
}

impl traits::Command for Command {
    /// Returns the address of the command
    fn address(self) -> u8 {
        self as u8
    }
}
