// //! A platform-agnostic driver for the Texas Instruments ADS1262/ADS1263
// //! 32-bit, 38-kSPS, delta-sigma ADC.
// //!
// //! This driver is built on top of the `embedded-hal` version 1.0 traits
// //! and is designed to be used with a shared SPI bus via the `SpiDevice` trait.
// //!
// //! # Usage with Shared SPI Bus
// //!
// //! 1.  Instantiate your SPI peripheral (`SpiBus`).
// //! 2.  Create a mutex to protect the SPI bus (e.g., `embassy_sync::mutex::Mutex`).
// //! 3.  Use a shared bus manager (like `embassy-embedded-hal::shared_bus`) to create
// //!     an `SpiDevice` for each ADC, providing the shared bus and the unique CS pin for each.
// //! 4.  Create a new driver instance for each `SpiDevice`.
// //!
// //! # Example (Conceptual)
// //!
// //! ```no_run
// //! # use core::cell::RefCell;
// //! # use embassy_sync::blocking_mutex::raw::NoopRawMutex;
// //! # use embassy_sync::blocking_mutex::Mutex;
// //! # use embedded_hal::delay::DelayNs;
// //! # use embedded_hal::digital::{InputPin, OutputPin};
// //! # use embedded_hal::spi::{SpiBus, SpiDevice};
// //! # use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice as SharedSpiDevice;
// //! #
// //! # // Dummy implementations for the example
// //! # #[derive(Debug)]
// //! # struct DummyError;
// //! # impl embedded_hal::spi::Error for DummyError {
// //! #     fn kind(&self) -> embedded_hal::spi::ErrorKind {
// //! #         embedded_hal::spi::ErrorKind::Other
// //! #     }
// //! # }
// //! # impl embedded_hal::digital::Error for DummyError {
// //! #     fn kind(&self) -> embedded_hal::digital::ErrorKind {
// //! #         embedded_hal::digital::ErrorKind::Other
// //! #     }
// //! # }
// //! # struct DummySpi;
// //! # impl embedded_hal::spi::ErrorType for DummySpi { type Error = DummyError; }
// //! # impl SpiBus<u8> for DummySpi {
// //! #     fn read(&mut self, _words: &mut [u8]) -> Result<(), Self::Error> { Ok(()) }
// //! #     fn write(&mut self, _words: &[u8]) -> Result<(), Self::Error> { Ok(()) }
// //! #     fn transfer(&mut self, _read: &mut [u8], _write: &[u8]) -> Result<(), Self::Error> { Ok(()) }
// //! #     fn transfer_in_place(&mut self, _words: &mut [u8]) -> Result<(), Self::Error> { Ok(()) }
// //! # }
// //! # struct DummyPin;
// //! # impl embedded_hal::digital::ErrorType for DummyPin { type Error = DummyError; }
// //! # impl OutputPin for DummyPin { fn set_low(&mut self) -> Result<(), DummyError> { Ok(()) } fn set_high(&mut self) -> Result<(), DummyError> { Ok(()) } }
// //! # impl InputPin for DummyPin { fn is_high(&mut self) -> Result<bool, DummyError> { Ok(false) } fn is_low(&mut self) -> Result<bool, DummyError> { Ok(true) } }
// //! # struct DummyDelay;
// //! # impl DelayNs for DummyDelay { fn delay_ns(&mut self, _ns: u32) {} }
// //! #
// //! # let spi_bus = Mutex::<NoopRawMutex, _>::new(RefCell::new(DummySpi));
// //! # let cs1 = DummyPin;
// //! # let mut rst1 = DummyPin;
// //! # let mut drdy1 = DummyPin;
// //! # let mut delay = DummyDelay;
// //! use ads1262_driver::{Ads1262, Command, Register};
// //!
// //! // 1. Create a Mutex-protected SPI bus
// //! let bus_mutex = Mutex::<NoopRawMutex, _>::new(RefCell::new(spi));
// //!
// //! // 2. Create an SpiDevice for the ADC
// //! let adc1_spi = SharedSpiDevice::new(&bus_mutex, cs1);
// //!
// //! // 3. Create a driver instance
// //! let mut adc1 = Ads1262::new(adc1_spi, rst1, drdy1);
// //!
// //! adc1.reset(&mut delay).unwrap();
// //! let id1 = adc1.read_id().unwrap();
// //! ```

// #![no_std]

// use embedded_hal_1::{
//     delay::DelayNs,
//     digital::{InputPin, OutputPin},
//     spi::{Operation, SpiDevice},
// };

// /// All possible errors in this crate
// #[derive(Debug)]
// pub enum Error<SpiE, PinE> {
//     /// SPI bus error
//     Spi(SpiE),
//     /// GPIO pin error
//     Pin(PinE),
//     /// Checksum mismatch on received data
//     Checksum,
// }

// /// ADS1262/ADS1263 driver
// pub struct Ads1262<SPI, RST, DRDY> {
//     spi: SPI,
//     rst: RST,
//     pub drdy: DRDY,
// }

// /// Commands for the ADS1262
// #[allow(dead_code)]
// #[derive(Clone, Copy)]
// pub enum Command {
//     NOP = 0x00,
//     RESET = 0x06,
//     START1 = 0x08,
//     STOP1 = 0x0A,
//     START2 = 0x0C,
//     STOP2 = 0x0E,
//     RDATA1 = 0x12,
//     RDATA2 = 0x14,
//     SYOCAL1 = 0x16,
//     SYGCAL1 = 0x17,
//     SFOCAL1 = 0x19,
//     SYOCAL2 = 0x1A,
//     SYGCAL2 = 0x1B,
//     SFOCAL2 = 0x1D,
//     RREG = 0x20,
//     WREG = 0x40,
// }

// /// Registers of the ADS1262
// #[allow(dead_code)]
// #[derive(Clone, Copy, PartialEq, Eq)]
// #[repr(u8)]
// pub enum Register {
//     ID = 0x00,
//     POWER = 0x01,
//     INTERFACE = 0x02,
//     MODE0 = 0x03,
//     MODE1 = 0x04,
//     MODE2 = 0x05,
//     INPMUX = 0x06,
//     OFCAL0 = 0x07,
//     OFCAL1 = 0x08,
//     OFCAL2 = 0x09,
//     FSCAL0 = 0x0A,
//     FSCAL1 = 0x0B,
//     FSCAL2 = 0x0C,
//     IDACMUX = 0x0D,
//     IDACMAG = 0x0E,
//     REFMUX = 0x0F,
//     TDACP = 0x10,
//     TDACN = 0x11,
//     GPIOCON = 0x12,
//     GPIODIR = 0x13,
//     GPIODAT = 0x14,
// }

// /// Contains constants for setting register values.
// pub mod register_data {
//     // POWER Register
//     pub const POWER_RESET: u8 = 1 << 4;
//     pub const POWER_VBIAS: u8 = 1 << 1;
//     pub const POWER_INTREF: u8 = 1 << 0;

//     // INTERFACE Register
//     pub const INTERFACE_TIMEOUT: u8 = 1 << 3;
//     pub const INTERFACE_STATUS: u8 = 1 << 2;
//     pub const INTERFACE_CRC_QUAD: u8 = 0b11;
//     pub const INTERFACE_CRC_XOR: u8 = 0b10;
//     pub const INTERFACE_CRC_NONE: u8 = 0b00;

//     // MODE0 Register
//     pub const MODE0_REFREV: u8 = 1 << 7;
//     pub const MODE0_RUNMODE_CONTINUOUS: u8 = 1 << 4;
//     pub const MODE0_CLK_EXT: u8 = 1 << 3;
//     pub const MODE0_DELAY_0: u8 = 0b000;
//     // ... add other delays

//     // MODE1 Register
//     pub const MODE1_FILTER_SINC1: u8 = 0b000 << 5;
//     pub const MODE1_FILTER_SINC2: u8 = 0b001 << 5;
//     pub const MODE1_FILTER_SINC3: u8 = 0b010 << 5;
//     pub const MODE1_FILTER_SINC4: u8 = 0b011 << 5;
//     pub const MODE1_FILTER_FIR: u8 = 0b100 << 5;
//     pub const MODE1_SB_ADC1_ONLY: u8 = 0b0000;
//     // ... add other SB settings

//     // MODE2 Register
//     pub const MODE2_BYPASS_PGA: u8 = 1 << 7;
//     pub const MODE2_GAIN_1: u8 = 0b000 << 4;
//     pub const MODE2_GAIN_2: u8 = 0b001 << 4;
//     pub const MODE2_GAIN_4: u8 = 0b010 << 4;
//     pub const MODE2_GAIN_8: u8 = 0b011 << 4;
//     pub const MODE2_GAIN_16: u8 = 0b100 << 4;
//     pub const MODE2_GAIN_32: u8 = 0b101 << 4;
//     pub const MODE2_SPS_2_5: u8 = 0b0000;
//     pub const MODE2_SPS_5: u8 = 0b0001;
//     pub const MODE2_SPS_10: u8 = 0b0010;
//     pub const MODE2_SPS_16_6: u8 = 0b0011;
//     pub const MODE2_SPS_20: u8 = 0b0100;
//     pub const MODE2_SPS_50: u8 = 0b0101;
//     pub const MODE2_SPS_60: u8 = 0b0110;
//     pub const MODE2_SPS_100: u8 = 0b0111;
//     // ... add other SPS settings

//     // INPMUX Register
//     pub const INPMUX_AIN0: u8 = 0b0000 << 4;
//     pub const INPMUX_AIN1: u8 = 0b0001 << 4;
//     // ... add other AIN positive inputs
//     pub const INPMUX_AINCOM_POS: u8 = 0b1010 << 4;

//     pub const INPMUX_AIN0_NEG: u8 = 0b0000;
//     pub const INPMUX_AIN1_NEG: u8 = 0b0001;
//     // ... add other AIN negative inputs
//     pub const INPMUX_AINCOM_NEG: u8 = 0b1010;
// }

// impl<SPI, RST, DRDY> Ads1262<SPI, RST, DRDY>
// where
//     SPI: SpiDevice,
//     RST: OutputPin,
//     DRDY: InputPin,
// {
//     /// Creates a new driver from an SpiDevice, RST pin, and DRDY pin.
//     pub fn new(spi: SPI, rst: RST, drdy: DRDY) -> Self {
//         Ads1262 { spi, rst, drdy }
//     }

//     /// Performs a hardware reset of the ADC.
//     pub fn reset(&mut self, delay: &mut impl DelayNs) -> Result<(), Error<SPI::Error, RST::Error>> {
//         self.rst.set_low().map_err(Error::Pin)?;
//         delay.delay_ms(10);
//         self.rst.set_high().map_err(Error::Pin)?;
//         delay.delay_ms(10);
//         Ok(())
//     }

//     /// Sends a command to the ADC.
//     pub fn send_command(&mut self, command: Command) -> Result<(), Error<SPI::Error, RST::Error>> {
//         self.spi.write(&[command as u8]).map_err(Error::Spi)
//     }

//     /// Reads a single register.
//     pub fn read_register(&mut self, reg: Register) -> Result<u8, Error<SPI::Error, RST::Error>> {
//         let mut buffer = [0u8; 1];
//         self.spi
//             .transaction(&mut [
//                 Operation::Write(&[Command::RREG as u8 | (reg as u8), 0x00]),
//                 Operation::Read(&mut buffer),
//             ])
//             .map_err(Error::Spi)?;
//         Ok(buffer[0])
//     }

//     /// Writes to a single register.
//     pub fn write_register(&mut self, reg: Register, data: u8) -> Result<(), Error<SPI::Error, RST::Error>> {
//         self.spi
//             .write(&[Command::WREG as u8 | (reg as u8), 0x00, data])
//             .map_err(Error::Spi)
//     }

//     /// Reads multiple registers starting from `reg`.
//     pub fn read_registers(
//         &mut self,
//         reg: Register,
//         count: u8,
//         buffer: &mut [u8],
//     ) -> Result<(), Error<SPI::Error, RST::Error>> {
//         assert!(buffer.len() >= count as usize);
//         self.spi
//             .transaction(&mut [
//                 Operation::Write(&[Command::RREG as u8 | (reg as u8), count - 1]),
//                 Operation::Read(&mut buffer[..count as usize]),
//             ])
//             .map_err(Error::Spi)
//     }

//     /// Writes to multiple registers starting from `reg`.
//     pub fn write_registers(&mut self, reg: Register, data: &[u8]) -> Result<(), Error<SPI::Error, RST::Error>> {
//         self.spi
//             .transaction(&mut [
//                 Operation::Write(&[Command::WREG as u8 | (reg as u8), data.len() as u8 - 1]),
//                 Operation::Write(data),
//             ])
//             .map_err(Error::Spi)
//     }

//     /// Reads the device ID.
//     pub fn read_id(&mut self) -> Result<u8, Error<SPI::Error, RST::Error>> {
//         self.read_register(Register::ID)
//     }

//     /// Reads the conversion data.
//     /// Returns a tuple of (status, data).
//     pub fn read_data(&mut self) -> Result<(Option<u8>, i32), Error<SPI::Error, RST::Error>> {
//         let interface_reg = self.read_register(Register::INTERFACE)?;
//         let status_enabled = (interface_reg & register_data::INTERFACE_STATUS) != 0;
//         let crc_enabled = (interface_reg & 0b11) != register_data::INTERFACE_CRC_NONE;

//         let mut read_len = 4;
//         if status_enabled {
//             read_len += 1;
//         }
//         if crc_enabled {
//             read_len += 1;
//         }

//         let mut buffer = [0u8; 6];
//         let read_slice = &mut buffer[..read_len];

//         self.spi
//             .transaction(&mut [
//                 Operation::Write(&[Command::RDATA1 as u8]),
//                 Operation::Read(read_slice),
//             ])
//             .map_err(Error::Spi)?;

//         let mut current_pos = 0;
//         let status = if status_enabled {
//             let s = read_slice[current_pos];
//             current_pos += 1;
//             Some(s)
//         } else {
//             None
//         };

//         let data_bytes = &read_slice[current_pos..current_pos + 4];
//         let data = i32::from_be_bytes(data_bytes.try_into().unwrap());
//         current_pos += 4;

//         if crc_enabled {
//             let received_crc = read_slice[current_pos];
//             let crc_data_start = if status_enabled { 0 } else { 1 };
//             let calculated_crc = crc8(&read_slice[crc_data_start..current_pos]);
//             if received_crc != calculated_crc {
//                 return Err(Error::Checksum);
//             }
//         }

//         Ok((status, data))
//     }
// }

// /// CRC-8-ATM calculation (polynomial 0x07).
// fn crc8(data: &[u8]) -> u8 {
//     let mut crc: u8 = 0;
//     for byte in data {
//         crc ^= byte;
//         for _ in 0..8 {
//             if (crc & 0x80) != 0 {
//                 crc = (crc << 1) ^ 0x07;
//             } else {
//                 crc <<= 1;
//             }
//         }
//     }
//     crc
// }


//! A platform-agnostic driver for the Texas Instruments ADS1262/ADS1263
//! 32-bit, 38-kSPS, delta-sigma ADC.
//!
//! This driver is built on top of the `embedded-hal` version 1.0 traits
//! and is designed to be used with a shared SPI bus via the `SpiDevice` trait.
//!
//! # Usage with Shared SPI Bus
//!
//! 1.  Instantiate your SPI peripheral (`SpiBus`).
//! 2.  Create a mutex to protect the SPI bus (e.g., `embassy_sync::mutex::Mutex`).
//! 3.  Use a shared bus manager (like `embassy-embedded-hal::shared_bus`) to create
//!     an `SpiDevice` for each ADC, providing the shared bus and the unique CS pin for each.
//! 4.  Create a new driver instance for each `SpiDevice`.
//!
//! # Example (Conceptual)
//!
//! ```no_run
//! # use core::cell::RefCell;
//! # use embassy_sync::blocking_mutex::raw::NoopRawMutex;
//! # use embassy_sync::blocking_mutex::Mutex;
//! # use embedded_hal::delay::DelayNs;
//! # use embedded_hal::digital::{InputPin, OutputPin};
//! # use embedded_hal::spi::{SpiBus, SpiDevice};
//! # use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice as SharedSpiDevice;
//! #
//! # // Dummy implementations for the example
//! # #[derive(Debug)]
//! # struct DummyError;
//! # impl embedded_hal::spi::Error for DummyError {
//! #     fn kind(&self) -> embedded_hal::spi::ErrorKind {
//! #         embedded_hal::spi::ErrorKind::Other
//! #     }
//! # }
//! # impl embedded_hal::digital::Error for DummyError {
//! #     fn kind(&self) -> embedded_hal::digital::ErrorKind {
//! #         embedded_hal::digital::ErrorKind::Other
//! #     }
//! # }
//! # struct DummySpi;
//! # impl embedded_hal::spi::ErrorType for DummySpi { type Error = DummyError; }
//! # impl SpiBus<u8> for DummySpi {
//! #     fn read(&mut self, _words: &mut [u8]) -> Result<(), Self::Error> { Ok(()) }
//! #     fn write(&mut self, _words: &[u8]) -> Result<(), Self::Error> { Ok(()) }
//! #     fn transfer(&mut self, _read: &mut [u8], _write: &[u8]) -> Result<(), Self::Error> { Ok(()) }
//! #     fn transfer_in_place(&mut self, _words: &mut [u8]) -> Result<(), Self::Error> { Ok(()) }
//! # }
//! # struct DummyPin;
//! # impl embedded_hal::digital::ErrorType for DummyPin { type Error = DummyError; }
//! # impl OutputPin for DummyPin { fn set_low(&mut self) -> Result<(), DummyError> { Ok(()) } fn set_high(&mut self) -> Result<(), DummyError> { Ok(()) } }
//! # impl InputPin for DummyPin { fn is_high(&mut self) -> Result<bool, DummyError> { Ok(false) } fn is_low(&mut self) -> Result<bool, DummyError> { Ok(true) } }
//! # struct DummyDelay;
//! # impl DelayNs for DummyDelay { fn delay_ns(&mut self, _ns: u32) {} }
//! #
//! # let spi_bus = Mutex::<NoopRawMutex, _>::new(RefCell::new(DummySpi));
//! # let cs1 = DummyPin;
//! # let mut rst1 = DummyPin;
//! # let mut drdy1 = DummyPin;
//! # let mut delay = DummyDelay;
//! use ads1262_driver::{Ads1262, Command, Register};
//!
//! // 1. Create a Mutex-protected SPI bus
//! let bus_mutex = Mutex::<NoopRawMutex, _>::new(RefCell::new(spi));
//!
//! // 2. Create an SpiDevice for the ADC
//! let adc1_spi = SharedSpiDevice::new(&bus_mutex, cs1);
//!
//! // 3. Create a driver instance
//! let mut adc1 = Ads1262::new(adc1_spi, rst1, drdy1);
//!
//! adc1.reset(&mut delay).unwrap();
//! let id1 = adc1.read_id().unwrap();
//! ```

#![no_std]

use embedded_hal_1::{
    delay::DelayNs,
    digital::{InputPin, OutputPin},
    spi::{Operation, SpiDevice},
};

/// All possible errors in this crate
#[derive(Debug)]
pub enum Error<SpiE, PinE> {
    /// SPI bus error
    Spi(SpiE),
    /// GPIO pin error
    Pin(PinE),
    /// Checksum mismatch on received data
    Checksum,
}

/// ADS1262/ADS1263 driver
pub struct Ads1262<SPI, RST, DRDY> {
    spi: SPI,
    rst: RST,
    pub drdy: DRDY,
}

/// Commands for the ADS1262
#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum Command {
    NOP = 0x00,
    RESET = 0x06,
    START1 = 0x08,
    STOP1 = 0x0A,
    START2 = 0x0C,
    STOP2 = 0x0E,
    RDATA1 = 0x12,
    RDATA2 = 0x14,
    SYOCAL1 = 0x16,
    SYGCAL1 = 0x17,
    SFOCAL1 = 0x19,
    SYOCAL2 = 0x1A,
    SYGCAL2 = 0x1B,
    SFOCAL2 = 0x1D,
    RREG = 0x20,
    WREG = 0x40,
}

/// Registers of the ADS1262
#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Register {
    ID = 0x00,
    POWER = 0x01,
    INTERFACE = 0x02,
    MODE0 = 0x03,
    MODE1 = 0x04,
    MODE2 = 0x05,
    INPMUX = 0x06,
    OFCAL0 = 0x07,
    OFCAL1 = 0x08,
    OFCAL2 = 0x09,
    FSCAL0 = 0x0A,
    FSCAL1 = 0x0B,
    FSCAL2 = 0x0C,
    IDACMUX = 0x0D,
    IDACMAG = 0x0E,
    REFMUX = 0x0F,
    TDACP = 0x10,
    TDACN = 0x11,
    GPIOCON = 0x12,
    GPIODIR = 0x13,
    GPIODAT = 0x14,
}

/// Contains constants for setting register values.
pub mod register_data {
    // POWER Register
    pub const POWER_RESET: u8 = 1 << 4;
    pub const POWER_VBIAS: u8 = 1 << 1;
    pub const POWER_INTREF: u8 = 1 << 0;
    pub const POWER_VBIAS_NONE: u8 = 0;

    // INTERFACE Register
    pub const INTERFACE_TIMEOUT: u8 = 1 << 3;
    pub const INTERFACE_STATUS: u8 = 1 << 2;
    pub const INTERFACE_CRC_QUAD: u8 = 0b11;
    pub const INTERFACE_CRC_XOR: u8 = 0b10;
    pub const INTERFACE_CRC_NONE: u8 = 0b00;

    // MODE0 Register
    pub const MODE0_REFREV: u8 = 1 << 7;
    pub const MODE0_RUNMODE_CONTINUOUS: u8 = 1 << 4;
    pub const MODE0_CLK_EXT: u8 = 1 << 3;
    pub const MODE0_DELAY_0: u8 = 0b000;
    // ... add other delays

    // MODE1 Register
    pub const MODE1_FILTER_SINC1: u8 = 0b000 << 5;
    pub const MODE1_FILTER_SINC2: u8 = 0b001 << 5;
    pub const MODE1_FILTER_SINC3: u8 = 0b010 << 5;
    pub const MODE1_FILTER_SINC4: u8 = 0b011 << 5;
    pub const MODE1_FILTER_FIR: u8 = 0b100 << 5;
    pub const MODE1_SB_ADC1_ONLY: u8 = 0b0000;
    // ... add other SB settings

    // MODE2 Register
    pub const MODE2_BYPASS_PGA: u8 = 1 << 7;
    pub const MODE2_GAIN_1: u8 = 0b000 << 4;
    pub const MODE2_GAIN_2: u8 = 0b001 << 4;
    pub const MODE2_GAIN_4: u8 = 0b010 << 4;
    pub const MODE2_GAIN_8: u8 = 0b011 << 4;
    pub const MODE2_GAIN_16: u8 = 0b100 << 4;
    pub const MODE2_GAIN_32: u8 = 0b101 << 4;
    pub const MODE2_SPS_2_5: u8 = 0b0000;
    pub const MODE2_SPS_5: u8 = 0b0001;
    pub const MODE2_SPS_10: u8 = 0b0010;
    pub const MODE2_SPS_16_6: u8 = 0b0011;
    pub const MODE2_SPS_20: u8 = 0b0100;
    pub const MODE2_SPS_50: u8 = 0b0101;
    pub const MODE2_SPS_60: u8 = 0b0110;
    pub const MODE2_SPS_100: u8 = 0b0111;
    // ... add other SPS settings

    // INPMUX Register
    pub const INPMUX_AIN0_POS: u8 = 0b0000 << 4;
    pub const INPMUX_AIN1_POS: u8 = 0b0001 << 4;
    pub const INPMUX_AIN2_POS: u8 = 0b0010 << 4;
    // ... add other AIN positive inputs
    pub const INPMUX_AINCOM_POS: u8 = 0b1010 << 4;

    pub const INPMUX_AIN0_NEG: u8 = 0b0000;
    pub const INPMUX_AIN1_NEG: u8 = 0b0001;
    pub const INPMUX_AIN2_NEG: u8 = 0b0010;
    pub const INPMUX_AIN3_NEG: u8 = 0b0011;
    // ... add other AIN negative inputs
    pub const INPMUX_AINCOM_NEG: u8 = 0b1010;

    // REFMUX Register
    pub const REFMUX_INTERNAL_2_5V_POS: u8 = 0b000 << 5;
    pub const REFMUX_EXTERNAL_AIN0_POS: u8 = 0b001 << 5;
    pub const REFMUX_EXTERNAL_AIN2_POS: u8 = 0b010 << 5;
    pub const REFMUX_AVDD_POS: u8 = 0b101 << 5;

    pub const REFMUX_INTERNAL_2_5V_NEG: u8 = 0b000 << 2;
    pub const REFMUX_EXTERNAL_AIN1_NEG: u8 = 0b001 << 2;
    pub const REFMUX_EXTERNAL_AIN3_NEG: u8 = 0b011 << 2;
    pub const REFMUX_AVSS_NEG: u8 = 0b101 << 2;
}

impl<SPI, RST, DRDY> Ads1262<SPI, RST, DRDY>
where
    SPI: SpiDevice,
    RST: OutputPin,
    DRDY: InputPin,
{
    /// Creates a new driver from an SpiDevice, RST pin, and DRDY pin.
    pub fn new(spi: SPI, rst: RST, drdy: DRDY) -> Self {
        Ads1262 { spi, rst, drdy }
    }

    /// Performs a hardware reset of the ADC.
    pub fn reset(&mut self, delay: &mut impl DelayNs) -> Result<(), Error<SPI::Error, RST::Error>> {
        self.rst.set_low().map_err(Error::Pin)?;
        delay.delay_us(10_000); // delay_ms(10)
        self.rst.set_high().map_err(Error::Pin)?;
        delay.delay_us(10_000); // delay_ms(10)
        Ok(())
    }

    /// Sends a command to the ADC.
    pub fn send_command(&mut self, command: Command) -> Result<(), Error<SPI::Error, RST::Error>> {
        self.spi.write(&[command as u8]).map_err(Error::Spi)
    }

    /// Reads a single register.
    pub fn read_register(&mut self, reg: Register) -> Result<u8, Error<SPI::Error, RST::Error>> {
        let mut buffer = [0u8; 1];
        self.spi
            .transaction(&mut [
                Operation::Write(&[Command::RREG as u8 | (reg as u8), 0x00]),
                Operation::Read(&mut buffer),
            ])
            .map_err(Error::Spi)?;
        Ok(buffer[0])
    }

    /// Writes to a single register.
    pub fn write_register(&mut self, reg: Register, data: u8) -> Result<(), Error<SPI::Error, RST::Error>> {
        self.spi
            .write(&[Command::WREG as u8 | (reg as u8), 0x00, data])
            .map_err(Error::Spi)
    }

    /// Reads multiple registers starting from `reg`.
    pub fn read_registers(
        &mut self,
        reg: Register,
        count: u8,
        buffer: &mut [u8],
    ) -> Result<(), Error<SPI::Error, RST::Error>> {
        assert!(buffer.len() >= count as usize);
        self.spi
            .transaction(&mut [
                Operation::Write(&[Command::RREG as u8 | (reg as u8), count - 1]),
                Operation::Read(&mut buffer[..count as usize]),
            ])
            .map_err(Error::Spi)
    }

    /// Writes to multiple registers starting from `reg`.
    pub fn write_registers(&mut self, reg: Register, data: &[u8]) -> Result<(), Error<SPI::Error, RST::Error>> {
        self.spi
            .transaction(&mut [
                Operation::Write(&[Command::WREG as u8 | (reg as u8), data.len() as u8 - 1]),
                Operation::Write(data),
            ])
            .map_err(Error::Spi)
    }

    /// Reads the device ID.
    pub fn read_id(&mut self) -> Result<u8, Error<SPI::Error, RST::Error>> {
        self.read_register(Register::ID)
    }

    /// Reads the conversion data.
    /// Returns a tuple of (status, data).
    pub fn read_data(&mut self) -> Result<(Option<u8>, i32), Error<SPI::Error, RST::Error>> {
        let interface_reg = self.read_register(Register::INTERFACE)?;
        let status_enabled = (interface_reg & register_data::INTERFACE_STATUS) != 0;
        let crc_enabled = (interface_reg & 0b11) != register_data::INTERFACE_CRC_NONE;

        let mut read_len = 4;
        if status_enabled {
            read_len += 1;
        }
        if crc_enabled {
            read_len += 1;
        }

        let mut buffer = [0u8; 6];
        let read_slice = &mut buffer[..read_len];

        self.spi
            .transaction(&mut [
                Operation::Write(&[Command::RDATA1 as u8]),
                Operation::Read(read_slice),
            ])
            .map_err(Error::Spi)?;

        let mut current_pos = 0;
        let status = if status_enabled {
            let s = read_slice[current_pos];
            current_pos += 1;
            Some(s)
        } else {
            None
        };

        let data_bytes = &read_slice[current_pos..current_pos + 4];
        let data = i32::from_be_bytes(data_bytes.try_into().unwrap());
        current_pos += 4;

        if crc_enabled {
            let received_crc = read_slice[current_pos];
            let crc_data_start = if status_enabled { 0 } else { 1 };
            let calculated_crc = crc8(&read_slice[crc_data_start..current_pos]);
            if received_crc != calculated_crc {
                return Err(Error::Checksum);
            }
        }

        Ok((status, data))
    }
}

/// CRC-8-ATM calculation (polynomial 0x07).
fn crc8(data: &[u8]) -> u8 {
    let mut crc: u8 = 0;
    for byte in data {
        crc ^= byte;
        for _ in 0..8 {
            if (crc & 0x80) != 0 {
                crc = (crc << 1) ^ 0x07;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}
