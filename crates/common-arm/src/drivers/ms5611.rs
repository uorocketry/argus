//! Driver for the MS5611 Barometric Pressure Sensor
use embedded_hal::{delay::DelayNs, digital::OutputPin, spi::SpiBus};

// According to datasheet section 4.1
mod command {
    pub const RESET: u8 = 0x1E;
    pub const ADC_READ: u8 = 0x00;
    // Convert D1 (Pressure)
    pub const CONVERT_D1_OSR_256: u8 = 0x40;
    pub const CONVERT_D1_OSR_512: u8 = 0x42;
    pub const CONVERT_D1_OSR_1024: u8 = 0x44;
    pub const CONVERT_D1_OSR_2048: u8 = 0x46;
    pub const CONVERT_D1_OSR_4096: u8 = 0x48;
    // Convert D2 (Temperature)
    pub const CONVERT_D2_OSR_256: u8 = 0x50;
    pub const CONVERT_D2_OSR_512: u8 = 0x52;
    pub const CONVERT_D2_OSR_1024: u8 = 0x54;
    pub const CONVERT_D2_OSR_2048: u8 = 0x56;
    pub const CONVERT_D2_OSR_4096: u8 = 0x58;
    // PROM Read
    // pub const PROM_READ_ADDR_0: u8 = 0xA0; // Reserved
    pub const PROM_READ_ADDR_1: u8 = 0xA2; // C1 Pressure sensitivity (SENST1)
    pub const PROM_READ_ADDR_2: u8 = 0xA4; // C2 Pressure offset (OFFT1)
    pub const PROM_READ_ADDR_3: u8 = 0xA6; // C3 Temperature coefficient of pressure sensitivity (TCS)
    pub const PROM_READ_ADDR_4: u8 = 0xA8; // C4 Temperature coefficient of pressure offset (TCO)
    pub const PROM_READ_ADDR_5: u8 = 0xAA; // C5 Reference temperature (TREF)
    pub const PROM_READ_ADDR_6: u8 = 0xAC; // C6 Temperature coefficient of the temperature (TEMPSENS)
                                           // pub const PROM_READ_ADDR_7: u8 = 0xAE; // C7 Serial Code & CRC
}

/// Oversampling Ratio (OSR) options
/// Higher OSR means higher precision but longer conversion times and higher power consumption.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OversamplingRatio {
    /// 256 samples, ~0.60 ms conversion time
    Osr256,
    /// 512 samples, ~1.17 ms conversion time
    Osr512,
    /// 1024 samples, ~2.28 ms conversion time
    Osr1024,
    /// 2048 samples, ~4.54 ms conversion time
    Osr2048,
    /// 4096 samples, ~9.04 ms conversion time
    Osr4096,
}

impl OversamplingRatio {
    /// Get the command byte for starting a pressure (D1) conversion
    fn pressure_command(self) -> u8 {
        match self {
            OversamplingRatio::Osr256 => command::CONVERT_D1_OSR_256,
            OversamplingRatio::Osr512 => command::CONVERT_D1_OSR_512,
            OversamplingRatio::Osr1024 => command::CONVERT_D1_OSR_1024,
            OversamplingRatio::Osr2048 => command::CONVERT_D1_OSR_2048,
            OversamplingRatio::Osr4096 => command::CONVERT_D1_OSR_4096,
        }
    }

    /// Get the command byte for starting a temperature (D2) conversion
    fn temperature_command(self) -> u8 {
        match self {
            OversamplingRatio::Osr256 => command::CONVERT_D2_OSR_256,
            OversamplingRatio::Osr512 => command::CONVERT_D2_OSR_512,
            OversamplingRatio::Osr1024 => command::CONVERT_D2_OSR_1024,
            OversamplingRatio::Osr2048 => command::CONVERT_D2_OSR_2048,
            OversamplingRatio::Osr4096 => command::CONVERT_D2_OSR_4096,
        }
    }

    /// Get the maximum conversion time in microseconds (based on datasheet max values)
    fn conversion_time_us(self) -> u32 {
        match self {
            OversamplingRatio::Osr256 => 600,
            OversamplingRatio::Osr512 => 1_170,
            OversamplingRatio::Osr1024 => 2_280,
            OversamplingRatio::Osr2048 => 4_540,
            OversamplingRatio::Osr4096 => 9_040,
        }
    }
}

/// Calibration Coefficients read from PROM
#[derive(Debug, Clone, Copy)]
struct CalibrationCoefficients {
    /// C1: Pressure sensitivity (SENST1)
    c1_sens_t1: u16,
    /// C2: Pressure offset (OFFT1)
    c2_off_t1: u16,
    /// C3: Temperature coefficient of pressure sensitivity (TCS)
    c3_tcs: u16,
    /// C4: Temperature coefficient of pressure offset (TCO)
    c4_tco: u16,
    /// C5: Reference temperature (TREF)
    c5_t_ref: u16,
    /// C6: Temperature coefficient of the temperature (TEMPSENS)
    c6_temp_sens: u16,
    // C7 (Serial/CRC) is read but maybe only used for CRC check, not stored here yet
}

/// MS5611 Driver Error
#[derive(Debug)]
pub enum Error<SPIE, CSE> {
    /// SPI communication error
    Spi(SPIE),
    /// Chip Select pin error
    Cs(CSE),
    /// CRC check failed on PROM data (Not yet implemented)
    CrcError,
    /// Calculation resulted in an invalid value (e.g. NaN or Infinity)
    /// This might indicate issues with raw data or coefficients.
    CalculationFault,
}

/// MS5611 Driver
pub struct Ms5611<SPI, CS, DELAY> {
    spi: SPI,
    cs: CS,
    delay: DELAY,
    coefficients: CalibrationCoefficients,
}

// Helper macro for handling CS pin toggling
macro_rules! with_cs {
    ($self:expr, $block:expr) => {{
        // Set CS low to start transaction
        $self.cs.set_low().map_err(Error::Cs)?;
        // Small delay might be needed depending on SPI speed and device, though often not.
        // $self.delay.delay_us(1);
        let result = $block;
        // Set CS high to end transaction
        $self.cs.set_high().map_err(Error::Cs)?;
        result
    }};
}

impl<SPI, CS, DELAY, SPIE, CSE> Ms5611<SPI, CS, DELAY>
where
    SPI: SpiBus<Error = SPIE>,
    CS: OutputPin<Error = CSE>,
    DELAY: DelayNs,
{
    /// Creates a new MS5611 driver instance.
    /// Performs a reset, waits, and reads calibration coefficients from the PROM.
    pub fn new(spi: SPI, mut cs: CS, mut delay: DELAY) -> Result<Self, Error<SPIE, CSE>> {
        // Ensure CS is high initially
        cs.set_high().map_err(Error::Cs)?;
        delay.delay_us(100); // Small delay after power-up before reset

        let mut sensor = Self {
            spi,
            cs,
            delay,
            // Placeholder coefficients, will be overwritten
            coefficients: CalibrationCoefficients {
                c1_sens_t1: 0,
                c2_off_t1: 0,
                c3_tcs: 0,
                c4_tco: 0,
                c5_t_ref: 0,
                c6_temp_sens: 0,
            },
        };

        sensor.reset()?;
        // Datasheet: Wait 2.8 ms (max) after reset
        sensor.delay.delay_us(3000);

        sensor.coefficients = sensor.read_coefficients()?;

        // Optional: Implement CRC check here using PROM word 7 (0xAE)

        Ok(sensor)
    }

    /// Sends the Reset command to the sensor.
    fn reset(&mut self) -> Result<(), Error<SPIE, CSE>> {
        with_cs!(self, {
            self.spi.write(&[command::RESET]).map_err(Error::Spi)
        })
    }

    /// Reads a 16-bit word from the specified PROM address.
    ///
    /// This function is corrected to use the `embedded-hal` v1.0.0 `SpiBus::transfer` API properly.
    fn read_prom_word(&mut self, address_command: u8) -> Result<u16, Error<SPIE, CSE>> {
        with_cs!(self, {
            // To read the 16-bit (2-byte) PROM value, we perform a single SPI transaction.
            // We need to send 3 bytes in total to receive the 2 bytes of data.
            //
            // The transaction on the bus looks like this:
            // MOSI: [address_command, dummy_byte_1, dummy_byte_2]
            // MISO: [   garbage_byte,   data_byte_1,  data_byte_2]
            //
            // We use one `transfer` call for this. The first byte we receive while sending
            // the command is ignored.

            // The buffer of data to send: command followed by two dummy bytes.
            let write_buffer = [address_command, 0x00, 0x00];
            // A buffer to store the 3 bytes we receive.
            let mut read_buffer = [0u8; 3];

            // Perform the simultaneous write and read operation.
            self.spi
                .transfer(&mut read_buffer, &write_buffer)
                .map_err(Error::Spi)?;

            // The 16-bit result is in the second and third bytes of the read buffer.
            // We construct the u16 value from these bytes in big-endian order.
            Ok(u16::from_be_bytes([read_buffer[1], read_buffer[2]]))
        })
    }

    /// Reads all calibration coefficients (C1-C6) from the PROM.
    fn read_coefficients(&mut self) -> Result<CalibrationCoefficients, Error<SPIE, CSE>> {
        // Note: PROM address 0 (0xA0) is reserved, address 7 (0xAE) is CRC/Serial
        Ok(CalibrationCoefficients {
            c1_sens_t1: self.read_prom_word(command::PROM_READ_ADDR_1)?,
            c2_off_t1: self.read_prom_word(command::PROM_READ_ADDR_2)?,
            c3_tcs: self.read_prom_word(command::PROM_READ_ADDR_3)?,
            c4_tco: self.read_prom_word(command::PROM_READ_ADDR_4)?,
            c5_t_ref: self.read_prom_word(command::PROM_READ_ADDR_5)?,
            c6_temp_sens: self.read_prom_word(command::PROM_READ_ADDR_6)?,
        })
    }

    /// Sends a conversion command (Pressure or Temperature).
    fn start_conversion(&mut self, command: u8) -> Result<(), Error<SPIE, CSE>> {
        with_cs!(self, { self.spi.write(&[command]).map_err(Error::Spi) })
    }

    // /// Reads the 24-bit raw ADC result from the sensor.
    // pub fn read_adc_raw(&mut self) -> Result<u32, Error<SPIE, CSE>> {
    //     with_cs!(self, {
    //         // Send ADC Read command (0x00) to clock out the data
    //         let mut buffer = [command::ADC_READ, 0x00, 0x00, 0x00]; // Send read cmd, receive 3 bytes
    //         self.spi.transfer(&mut buffer).map_err(Error::Spi)?;
    //         Ok(u32::from_be_bytes([0, buffer[1], buffer[2], buffer[3]])) // Pad to 4 bytes for u32
    //     })
    // }

    /// Reads the 24-bit raw ADC result from the sensor.
    ///
    /// This function is corrected to use the `embedded-hal` v1.0.0 `SpiBus::transfer` API properly.
    fn read_adc_raw(&mut self) -> Result<u32, Error<SPIE, CSE>> {
        with_cs!(self, {
            // To read the 24-bit (3-byte) ADC value, we need to send a 4-byte sequence.
            // The first byte is the ADC_READ command, followed by three dummy bytes
            // to provide the clock signals needed to read the 3 bytes of data.
            //
            // The transaction on the bus looks like this:
            // MOSI: [ADC_READ_CMD,      dummy_byte_1,  dummy_byte_2,  dummy_byte_3]
            // MISO: [  garbage_byte,     data_byte_1,   data_byte_2,   data_byte_3]
            //
            // We use a single `transfer` call for this entire operation.

            // The buffer of data to send.
            let write_buffer = [command::ADC_READ, 0x00, 0x00, 0x00];
            // A buffer to store the 4 bytes we receive.
            let mut read_buffer = [0u8; 4];

            // Perform the simultaneous write and read operation.
            self.spi
                .transfer(&mut read_buffer, &write_buffer)
                .map_err(Error::Spi)?;

            // The 24-bit result is in the last three bytes of the read buffer.
            // We construct a u32 value from these bytes, padding the most significant byte with 0.
            Ok(u32::from_be_bytes([
                0,
                read_buffer[1],
                read_buffer[2],
                read_buffer[3],
            ]))
        })
    }

    /// Reads the raw temperature value (D2).
    /// Starts conversion, waits, and reads the ADC.
    pub fn read_raw_temperature(
        &mut self,
        osr: OversamplingRatio,
    ) -> Result<u32, Error<SPIE, CSE>> {
        self.start_conversion(osr.temperature_command())?;
        self.delay.delay_us(osr.conversion_time_us());
        self.read_adc_raw()
    }

    /// Reads the raw pressure value (D1).
    /// Starts conversion, waits, and reads the ADC.
    pub fn read_raw_pressure(&mut self, osr: OversamplingRatio) -> Result<u32, Error<SPIE, CSE>> {
        self.start_conversion(osr.pressure_command())?;
        self.delay.delay_us(osr.conversion_time_us());
        self.read_adc_raw()
    }

    /// Performs a full temperature and pressure reading cycle and returns compensated values.
    /// Reads temperature (D2), then pressure (D1), then performs calculations.
    ///
    /// Returns `(temperature_celsius, pressure_kpa)`
    pub fn read_pressure_temperature(
        &mut self,
        osr: OversamplingRatio,
    ) -> Result<(f32, f32), Error<SPIE, CSE>> {
        let d2_raw = self.read_raw_temperature(osr)?;
        let d1_raw = self.read_raw_pressure(osr)?;

        self.calculate_compensated_values(d1_raw, d2_raw)
    }

    /// Calculates compensated temperature and pressure using raw ADC values and PROM coefficients.
    /// Implements the 1st and 2nd order compensation formulas from the datasheet.
    ///
    /// Returns `(temperature_celsius, pressure_kpa)`
    fn calculate_compensated_values(
        &self,
        d1_raw: u32, // Raw Pressure
        d2_raw: u32, // Raw Temperature
    ) -> Result<(f32, f32), Error<SPIE, CSE>> {
        let c = &self.coefficients;

        // Cast coefficients to i64 for intermediate calculations to prevent overflow
        let c1 = c.c1_sens_t1 as i64;
        let c2 = c.c2_off_t1 as i64;
        let c3 = c.c3_tcs as i64;
        let c4 = c.c4_tco as i64;
        let c5 = c.c5_t_ref as i64;
        let c6 = c.c6_temp_sens as i64;
        let d1 = d1_raw as i64;
        let d2 = d2_raw as i64;

        // --- First Order Calculation ---
        // dT = D2 - C5 * 2^8
        let dt = d2 - (c5 << 8);

        // TEMP = 2000 + dT * C6 / 2^23  (Result in 0.01 degC)
        let temp_i32 = (2000 + ((dt * c6) >> 23)) as i32; // Cast to i32 for checks

        // OFF = C2 * 2^16 + (C4 * dT) / 2^7
        let off = (c2 << 16) + ((c4 * dt) >> 7);

        // SENS = C1 * 2^15 + (C3 * dT) / 2^8
        let sens = (c1 << 15) + ((c3 * dt) >> 8);

        // --- Second Order Temperature Compensation ---
        let mut temp = temp_i32 as i64; // Use i64 for further calculations
        let mut off2 = 0i64;
        let mut sens2 = 0i64;
        let mut t2 = 0i64;

        if temp < 2000 {
            // T2 = dT^2 / 2^31
            t2 = (dt * dt) >> 31;

            // OFF2 = 5 * (TEMP - 2000)^2 / 2
            let temp_diff_sq = (temp - 2000) * (temp - 2000);
            off2 = 5 * temp_diff_sq >> 1; // Divide by 2

            // SENS2 = 5 * (TEMP - 2000)^2 / 4
            sens2 = 5 * temp_diff_sq >> 2; // Divide by 4

            if temp < -1500 {
                // OFF2 = OFF2 + 7 * (TEMP + 1500)^2
                let temp_low_diff_sq = (temp + 1500) * (temp + 1500);
                off2 += 7 * temp_low_diff_sq;
                // SENS2 = SENS2 + 11 * (TEMP + 1500)^2 / 2
                sens2 += 11 * temp_low_diff_sq >> 1; // Divide by 2
            }
        }

        // Apply second order corrections
        temp -= t2;
        let off_compensated = off - off2;
        let sens_compensated = sens - sens2;

        // --- Final Pressure Calculation ---
        // P = (D1 * SENS / 2^21 - OFF) / 2^15 (Result in 0.01 mbar)
        let p_i32 = (((d1 * sens_compensated) >> 21) - off_compensated) >> 15; // Cast to i32

        // Convert to final units (float)
        // Handle potential division by zero or NaN/Infinity results from float conversion
        let temp_celsius = temp as f32 / 100.0;
        // p_i32 is in 0.01 mbar. 1 mbar = 0.1 kPa. So p_i32 / 1000.0 gives kPa.
        let pressure_kpa = p_i32 as f32 / 1000.0;

        // Check for NaN or Infinity which might occur if intermediate calculations were extreme
        if temp_celsius.is_finite() && pressure_kpa.is_finite() {
            Ok((temp_celsius, pressure_kpa))
        } else {
            Err(Error::CalculationFault)
        }
    }
}
