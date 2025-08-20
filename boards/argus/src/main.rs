#![feature(impl_trait_in_assoc_type)]
#![no_std]
#![no_main]

#[cfg(not(any(feature = "pressure", feature = "temperature", feature = "strain")))]
compile_error!(
    "You must enable exactly one of the features: 'pressure', 'temperature', or 'strain'."
);

// mod adc_manager;
mod ads;
mod resources;
mod sd;
mod state_machine;

#[cfg(feature = "calibration")]
mod calibration;

use crate::resources::{ADC_SPI_BUS_CELL, HEAP};
use crate::state_machine::{sm_task, StateMachine};
use core::cell::RefCell;
use core::marker::PhantomData;
use defmt::*;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::adc::Adc;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Level, Output, Pull, Speed};
use embassy_stm32::mode::Blocking;
use embassy_stm32::rtc::Rtc;
use embassy_stm32::spi::{Config as SpiConfig, Spi};
use embassy_stm32::time::{mhz, Hertz};
use embassy_stm32::{can, mode, peripherals, rcc};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_time::Instant;
use embassy_time::{Delay, Duration, Ticker, Timer};
use embedded_hal_1::digital::{InputPin, OutputPin};
use embedded_hal_1::spi::SpiDevice;
use embedded_hal_bus::spi::RefCellDevice;
use libm::logf;
use messages_prost::prost::Message;
use panic_probe as _;
use pid::Pid;
use static_cell::StaticCell;

use crate::resources::SD_CHANNEL;
// Use the asynchronous SpiDevice from embassy-embedded-hal

use crate::ads::register_data;
use crate::ads::{Ads1262, Command, Register};

/// The target temperature we want to maintain.
const SETPOINT_TEMP_C: f32 = 25.0;

/// How often the PID control loop runs.
const CONTROL_LOOP_INTERVAL_MS: u64 = 1000;

// The maximum raw value for the ADC (2^12 - 1 for a 12-bit ADC).
const ADC_MAX_VALUE: f32 = 4095.0;

// The value of the fixed resistor in your voltage divider circuit (in Ohms).
// This is taken from the schematic (R5 = 1.6kΩ).
const DIVIDER_RESISTANCE: f32 = 1600.0;

// --- NTC Thermistor Datasheet Parameters ---
// IMPORTANT: You MUST get these values from the datasheet for YOUR specific thermistor.
// These are common values for a standard 10k NTC thermistor.

/// Nominal resistance at the nominal temperature (e.g., 10kΩ at 25°C).
const THERMISTOR_NOMINAL_RESISTANCE: f32 = 10000.0;

/// The Beta coefficient of the thermistor (often in the range 3000-4500).
const THERMISTOR_BETA: f32 = 3950.0;

/// Nominal temperature in Kelvin (25°C).
const TEMPERATURE_NOMINAL_KELVIN: f32 = 298.15; // 25.0 + 273.15

// --- PID Tuning Constants ---
// You MUST tune these for your specific hardware setup.
const KP: f32 = 2.5;
const KI: f32 = 0.1;
const KD: f32 = 0.5;

// =================================================================================
// Shared Resources & Types
// =================================================================================

// static FAULT_CHANNEL: Channel<CriticalSectionRawMutex, , 2> = Channel::new();

// The SPI bus is protected by a Mutex, so the RefCell is not needed.
static SPI_BUS: StaticCell<embassy_sync::mutex::Mutex<CriticalSectionRawMutex, Spi<mode::Async>>> =
    StaticCell::new();

// Static variable for the RTC
pub static RTC: Mutex<CriticalSectionRawMutex, RefCell<Option<Rtc>>> =
    Mutex::new(RefCell::new(None));

pub struct TimeSink {
    _marker: PhantomData<*const ()>,
}

impl TimeSink {
    fn new() -> Self {
        TimeSink {
            _marker: PhantomData,
        }
    }
}

impl embedded_sdmmc::TimeSource for TimeSink {
    fn get_timestamp(&self) -> embedded_sdmmc::Timestamp {
        embedded_sdmmc::Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

// =================================================================================
// Sensor Configuration Functions
// =================================================================================

/// Configures an ADS1262 for thermocouple measurement.
///
/// This setup uses the internal 2.5V reference and enables the bias voltage
/// which is useful for open-circuit detection.
///
/// - ADC Inputs: AIN0 (positive), AIN1 (negative)
/// - PGA Gain: 32
/// - Data Rate: 20 SPS (good for low noise)
pub fn configure_adc_for_thermocouple<SPI, RST, DRDY>(
    adc: &mut Ads1262<SPI, RST, DRDY>,
) -> Result<(), ads::Error<SPI::Error, <RST as embedded_hal_1::digital::ErrorType>::Error>>
where
    SPI: SpiDevice,
    RST: OutputPin,
    DRDY: InputPin,
{
    // Disable CRC and STATUS bytes for simplicity
    adc.write_register(Register::INTERFACE, register_data::INTERFACE_CRC_NONE)?;
    // Use internal reference, enable VBIAS for open-circuit detection
    adc.write_register(
        Register::POWER,
        register_data::POWER_INTREF | register_data::POWER_VBIAS,
    )?;
    // Set gain and data rate
    adc.write_register(
        Register::MODE2,
        register_data::MODE2_GAIN_32 | register_data::MODE2_SPS_20,
    )?;
    // Set input mux to AIN0 and AIN1
    adc.write_register(
        Register::INPMUX,
        register_data::INPMUX_AIN0_POS | register_data::INPMUX_AIN1_NEG,
    )?;
    Ok(())
}

/// Configures an ADS1262 for a strain gauge (Wheatstone bridge).
///
/// This setup is for ratiometric measurements, using the bridge's excitation
/// voltage as the reference.
///
/// - ADC Inputs: AIN2 (positive), AIN3 (negative)
/// - Reference Inputs: AVDD (positive), AVSS (negative)
/// - PGA Gain: 32 (Max for ADS1262)
/// - Data Rate: 100 SPS
pub fn configure_adc_for_strain_gauge<SPI, RST, DRDY>(
    adc: &mut Ads1262<SPI, RST, DRDY>,
) -> Result<(), ads::Error<SPI::Error, <RST as embedded_hal_1::digital::ErrorType>::Error>>
where
    SPI: SpiDevice,
    RST: OutputPin,
    DRDY: InputPin,
{
    // Disable CRC and STATUS bytes for simplicity
    adc.write_register(Register::INTERFACE, register_data::INTERFACE_CRC_NONE)?;
    // Disable internal reference
    adc.write_register(Register::POWER, 0x00)?;
    // Use AVDD/AVSS as reference for ratiometric measurement
    adc.write_register(
        Register::REFMUX,
        register_data::REFMUX_AVDD_POS | register_data::REFMUX_AVSS_NEG,
    )?;
    // Set gain and data rate. Strain gauges have small outputs, so max gain is often needed.
    adc.write_register(
        Register::MODE2,
        register_data::MODE2_GAIN_32 | register_data::MODE2_SPS_100,
    )?;
    // Set input mux to AIN2 and AIN3
    adc.write_register(
        Register::INPMUX,
        register_data::INPMUX_AIN2_POS | register_data::INPMUX_AIN3_NEG,
    )?;
    Ok(())
}

/// Configures an ADS1262 for a pressure sensor.
///
/// This configuration assumes a ratiometric bridge-type pressure sensor,
/// similar to a strain gauge.
pub fn configure_adc_for_pressure_sensor<SPI, RST, DRDY>(
    adc: &mut Ads1262<SPI, RST, DRDY>,
) -> Result<(), ads::Error<SPI::Error, <RST as embedded_hal_1::digital::ErrorType>::Error>>
where
    SPI: SpiDevice,
    RST: OutputPin,
    DRDY: InputPin,
{
    // Assuming a ratiometric bridge sensor, same as strain gauge
    configure_adc_for_strain_gauge(adc)
}

// =================================================================================
// Application Tasks
// =================================================================================

#[embassy_executor::task]
async fn led_blinker_task(pin: peripherals::PA3) {
    let mut led = Output::new(pin, Level::High, Speed::Low);
    info!("LED blinker task started.");
    loop {
        led.set_high();
        Timer::after_millis(500).await;
        led.set_low();
        Timer::after_millis(500).await;
    }
}

/// Converts a raw ADC reading from the thermistor's voltage divider
/// into a temperature in Celsius.
fn adc_to_celsius(adc_value: u16) -> f32 {
    // 1. Calculate the resistance of the thermistor using the voltage divider formula.
    let resistance = DIVIDER_RESISTANCE / ((ADC_MAX_VALUE / adc_value as f32) - 1.0);

    // 2. Calculate temperature using the Beta-parameter equation.
    let steinhart = logf(resistance / THERMISTOR_NOMINAL_RESISTANCE) / THERMISTOR_BETA
        + (1.0 / TEMPERATURE_NOMINAL_KELVIN);

    let temp_kelvin = 1.0 / steinhart;

    // 3. Convert from Kelvin to Celsius.
    let temp_celsius = temp_kelvin - 273.15;

    temp_celsius
}

/// Sets the heater state based on the PID controller's output.
fn set_heater_state(heater_pin: &mut Output, pid_output: f32) {
    if pid_output > 50.0 {
        heater_pin.set_high();
    } else {
        heater_pin.set_low();
    }
}

#[embassy_executor::task]
async fn temperature_regulator(
    mut adc: Adc<'static, embassy_stm32::peripherals::ADC1>,
    mut temp_pin: embassy_stm32::peripherals::PB1,
    mut heater_pin: Output<'static>,
) {
    defmt::info!("Temperature regulator task started.");

    // Configure the PID controller.
    let mut pid = Pid::new(SETPOINT_TEMP_C, 100.0);
    pid.p(KP, 100.0)
        .i(KI, 100.0) // Limit integral contribution to prevent wind-up
        .d(KD, 100.0);

    let mut ticker = Ticker::every(Duration::from_millis(CONTROL_LOOP_INTERVAL_MS));

    loop {
        // Read the raw ADC value from the thermistor pin.
        let adc_raw = adc.blocking_read(&mut temp_pin);
        info!("{}", adc_raw);

        // Convert the raw value to a temperature in Celsius.
        let measurement = adc_to_celsius(adc_raw);

        // Calculate the new control output.
        let control_output = pid.next_control_output(measurement);

        // Apply the new output to the heater pin (on/off).
        set_heater_state(&mut heater_pin, control_output.output);

        defmt::info!(
            "Setpoint: {}°C, Measured: {}°C -> PID Output: {} (P: {}, I: {}, D: {}) -> Heater: {}",
            SETPOINT_TEMP_C,
            measurement,
            control_output.output,
            control_output.p,
            control_output.i,
            control_output.d,
            if heater_pin.is_set_high() {
                "ON"
            } else {
                "OFF"
            }
        );

        // Wait for the next tick.
        ticker.next().await;
    }
}

/// Converts a raw 32-bit ADC value to voltage.
/// The ADS1262 is a 32-bit ADC, but the effective number of bits is less.
/// The output is a 32-bit two's complement integer.
/// Full-scale range is +/- VREF / GAIN.
const VREF_INTERNAL: f64 = 2.5;
fn adc_to_voltage(raw_data: i32, vref: f64, gain: u8) -> f64 {
    (raw_data as f64 / 2_147_483_647.0) * (vref / gain as f64)
}

#[embassy_executor::task]
async fn adc1_task(
    mut adc: Ads1262<
        RefCellDevice<'static, Spi<'static, Blocking>, Output<'static>, Delay>,
        Output<'static>,
        ExtiInput<'static>,
    >,
) {
    let mut sensor_id = 0;
    loop {
        // Wait for the DRDY pin to go low, indicating data is ready.
        adc.drdy.wait_for_low().await;

        let data = adc.read_data();
        if let Ok((_status, raw_data)) = data {
            info!("ADC1 Raw Data: {}", raw_data);
            #[cfg(feature = "temperature")]
            {
                let volts = adc_to_voltage(raw_data, VREF_INTERNAL, 32);
                info!("Voltage: {} V", volts);
                if let Some(celsius) = thermocouple_converter::voltage_to_celsius(volts) {
                    info!("Celsius: {} C", celsius);

                    let mut buf: [u8; 255] = [0; 255];
                    let msg = messages_prost::radio::RadioFrame {
                        node: messages_prost::common::Node::Phoenix.into(),
                        payload: Some(
                            messages_prost::radio::radio_frame::Payload::ArgusTemperature(
                                messages_prost::sensor::argus::Temperature {
                                    sensor_id,
                                    temperature: celsius,
                                },
                            ),
                        ),
                        millis_since_start: Instant::now().as_millis(),
                    };
                    msg.encode_length_delimited(&mut buf.as_mut())
                        .expect("Failed to encode SBG GPS Position");

                    SD_CHANNEL.send(("temperature.txt", buf)).await;
                }

                sensor_id += 1;
                // update the sensor_id, this is fugly yandre dev ah code
                // match sensor_id {
                //     0 => {
                //         adc.write_register(Register::INPMUX, register_data::INPMUX_AIN0_POS | register_data::INPMUX_AIN1_NEG);
                //     }
                //     1 => {
                //         adc.write_register(Register::INPMUX, register_data::INPMUX_AIN2_POS | register_data::INPMUX_AIN3_NEG);
                //     }
                //     2 => {
                //         adc.write_register(Register::INPMUX, register_data::INPMUX_AIN4_POS | register_data::INPMUX_AIN5_NEG);
                //     }
                //     3 => {
                //         adc.write_register(Register::INPMUX, register_data::INPMUX_AIN6_POS | register_data::INPMUX_AIN7_NEG);
                //     }
                //     _ => {
                //         sensor_id = 0;
                //         adc.write_register(Register::INPMUX, register_data::INPMUX_AIN0_POS | register_data::INPMUX_AIN1_NEG);
                //     }
                // }
            }

            #[cfg(feature = "pressure")]
            {
                // V_EXCITATION must be defined based on your hardware setup.
                const V_EXCITATION: f64 = 5.0;
                let volts = adc_to_voltage(raw_data, V_EXCITATION, 32);
                info!("Voltage: {} V", volts);
                let pressure: f64 = (10000.0 / ((60.0 / 100.0) * (5.0 / 3.0))) * volts;
                info!("Pressure (psi): {}", pressure);

                let mut buf: [u8; 255] = [0; 255];
                let msg = messages_prost::radio::RadioFrame {
                    node: messages_prost::common::Node::Phoenix.into(),
                    payload: Some(messages_prost::radio::radio_frame::Payload::ArgusPressure(
                        messages_prost::sensor::argus::Pressure {
                            sensor_id,
                            pressure,
                        },
                    )),
                    millis_since_start: Instant::now().as_millis(),
                };
                msg.encode_length_delimited(&mut buf.as_mut())
                    .expect("Failed to encode SBG GPS Position");

                SD_CHANNEL.send(("pressure.txt", buf)).await;
                sensor_id += 1;
                // update the sensor_id, this is fugly
                match sensor_id {
                    0 => {
                        adc.write_register(
                            Register::INPMUX,
                            register_data::INPMUX_AIN0_POS | register_data::INPMUX_AIN1_NEG,
                        );
                    }
                    1 => {
                        adc.write_register(
                            Register::INPMUX,
                            register_data::INPMUX_AIN2_POS | register_data::INPMUX_AIN3_NEG,
                        );
                    }
                    2 => {
                        adc.write_register(
                            Register::INPMUX,
                            register_data::INPMUX_AIN4_POS | register_data::INPMUX_AIN5_NEG,
                        );
                    }
                    3 => {
                        adc.write_register(
                            Register::INPMUX,
                            register_data::INPMUX_AIN6_POS | register_data::INPMUX_AIN7_NEG,
                        );
                    }
                    _ => {
                        sensor_id = 0;
                        adc.write_register(
                            Register::INPMUX,
                            register_data::INPMUX_AIN0_POS | register_data::INPMUX_AIN1_NEG,
                        );
                    }
                }
                Timer::after_millis(10).await;
            }

            #[cfg(feature = "strain")]
            {
                // V_EXCITATION must be defined based on your hardware setup.
                const V_EXCITATION: f64 = 5.0;
                let volts = adc_to_voltage(raw_data, V_EXCITATION, 32);
                info!("Voltage: {} V", volts);
                let strain = straingauge_converter::voltage_to_strain_full(volts, 2.0);
                info!("Strain: {}", strain);

                let mut buf: [u8; 255] = [0; 255];
                let msg = messages_prost::radio::RadioFrame {
                    node: messages_prost::common::Node::Phoenix.into(),
                    payload: Some(messages_prost::radio::radio_frame::Payload::ArgusStrain(
                        messages_prost::sensor::argus::Strain { sensor_id, strain },
                    )),
                    millis_since_start: Instant::now().as_millis(),
                };
                msg.encode_length_delimited(&mut buf.as_mut())
                    .expect("Failed to encode SBG GPS Position");

                SD_CHANNEL.send(("strain.txt", buf)).await;
                // sensor_id += 1;
                // // update the sensor_id, this is fugly
                // match sensor_id {
                //     0 => {
                //         adc.write_register(Register::INPMUX, register_data::INPMUX_AIN0_POS | register_data::INPMUX_AIN1_NEG);

                //     }
                //     1 => {
                //         adc.write_register(Register::INPMUX, register_data::INPMUX_AIN2_POS | register_data::INPMUX_AIN3_NEG);

                //     }
                //     2 => {
                //         adc.write_register(Register::INPMUX, register_data::INPMUX_AIN4_POS | register_data::INPMUX_AIN5_NEG);

                //     }
                //     3 => {
                //         adc.write_register(Register::INPMUX, register_data::INPMUX_AIN6_POS | register_data::INPMUX_AIN7_NEG);

                //     }
                //     4 => {
                //         adc.write_register(Register::INPMUX, register_data::INPMUX_AIN8_POS | register_data::INPMUX_AIN9_NEG);

                //     }
                //     _ => {
                //         sensor_id = 0;
                //         adc.write_register(Register::INPMUX, register_data::INPMUX_AIN0_POS | register_data::INPMUX_AIN1_NEG);
                //     }
                // }
            }
        } else {
            info!("Failed to read ADC1 data.");
        }
    }
}

#[embassy_executor::task]
async fn adc2_task(
    mut adc: Ads1262<
        RefCellDevice<'static, Spi<'static, Blocking>, Output<'static>, Delay>,
        Output<'static>,
        ExtiInput<'static>,
    >,
) {
    let mut sensor_id = 0;

    loop {
        // Wait for the DRDY pin to go low, indicating data is ready.
        adc.drdy.wait_for_low().await;

        let data = adc.read_data();
        if let Ok((_status, raw_data)) = data {
            info!("ADC2 Raw Data: {}", raw_data);

            #[cfg(feature = "temperature")]
            {
                let volts = adc_to_voltage(raw_data, VREF_INTERNAL, 32);
                info!("Voltage: {} V", volts);
                if let Some(celsius) = thermocouple_converter::voltage_to_celsius(volts) {
                    info!("Celsius: {} C", celsius);

                    let mut buf: [u8; 255] = [0; 255];
                    let msg = messages_prost::radio::RadioFrame {
                        node: messages_prost::common::Node::Phoenix.into(),
                        payload: Some(
                            messages_prost::radio::radio_frame::Payload::ArgusTemperature(
                                messages_prost::sensor::argus::Temperature {
                                    sensor_id,
                                    temperature: celsius,
                                },
                            ),
                        ),
                        millis_since_start: Instant::now().as_millis(),
                    };
                    msg.encode_length_delimited(&mut buf.as_mut())
                        .expect("Failed to encode SBG GPS Position");

                    SD_CHANNEL.send(("temperature.txt", buf)).await;
                }

                sensor_id += 1;
                // update the sensor_id, this is fugly
                match sensor_id {
                    0 => {
                        adc.write_register(
                            Register::INPMUX,
                            register_data::INPMUX_AIN0_POS | register_data::INPMUX_AIN1_NEG,
                        );
                    }
                    1 => {
                        adc.write_register(
                            Register::INPMUX,
                            register_data::INPMUX_AIN2_POS | register_data::INPMUX_AIN3_NEG,
                        );
                    }
                    2 => {
                        adc.write_register(
                            Register::INPMUX,
                            register_data::INPMUX_AIN4_POS | register_data::INPMUX_AIN5_NEG,
                        );
                    }
                    3 => {
                        adc.write_register(
                            Register::INPMUX,
                            register_data::INPMUX_AIN6_POS | register_data::INPMUX_AIN7_NEG,
                        );
                    }
                    _ => {
                        sensor_id = 0;
                        adc.write_register(
                            Register::INPMUX,
                            register_data::INPMUX_AIN0_POS | register_data::INPMUX_AIN1_NEG,
                        );
                    }
                }
            }

            #[cfg(feature = "pressure")]
            {
                // V_EXCITATION must be defined based on your hardware setup.
                const V_EXCITATION: f64 = 5.0;
                let volts = adc_to_voltage(raw_data, V_EXCITATION, 32);
                info!("Voltage: {} V", volts);
                let pressure: f64 = (10000.0 / ((60.0 / 100.0) * (5.0 / 3.0))) * volts;
                info!("Pressure (psi): {}", pressure);

                let mut buf: [u8; 255] = [0; 255];
                let msg = messages_prost::radio::RadioFrame {
                    node: messages_prost::common::Node::Phoenix.into(),
                    payload: Some(messages_prost::radio::radio_frame::Payload::ArgusPressure(
                        messages_prost::sensor::argus::Pressure {
                            sensor_id,
                            pressure,
                        },
                    )),
                    millis_since_start: Instant::now().as_millis(),
                };
                msg.encode_length_delimited(&mut buf.as_mut())
                    .expect("Failed to encode SBG GPS Position");

                SD_CHANNEL.send(("pressure.txt", buf)).await;
                // sensor_id += 1;
                // // update the sensor_id, this is fugly
                // match sensor_id {
                //     0 => {
                //         adc.write_register(Register::INPMUX, register_data::INPMUX_AIN0_POS | register_data::INPMUX_AIN1_NEG);

                //     }
                //     1 => {
                //         adc.write_register(Register::INPMUX, register_data::INPMUX_AIN2_POS | register_data::INPMUX_AIN3_NEG);

                //     }
                //     2 => {
                //         adc.write_register(Register::INPMUX, register_data::INPMUX_AIN4_POS | register_data::INPMUX_AIN5_NEG);

                //     }
                //     3 => {
                //         adc.write_register(Register::INPMUX, register_data::INPMUX_AIN6_POS | register_data::INPMUX_AIN7_NEG);

                //     }
                //     _ => {
                //         sensor_id = 0;
                //         adc.write_register(Register::INPMUX, register_data::INPMUX_AIN0_POS | register_data::INPMUX_AIN1_NEG);

                //     }
                // }
            }

            #[cfg(feature = "strain")]
            {
                // V_EXCITATION must be defined based on your hardware setup.
                const V_EXCITATION: f64 = 5.0;
                let volts = adc_to_voltage(raw_data, V_EXCITATION, 32);
                info!("Voltage: {} V", volts);
                let strain = straingauge_converter::voltage_to_strain_full(volts, 2.0);
                info!("Strain: {}", strain);

                let mut buf: [u8; 255] = [0; 255];
                let msg = messages_prost::radio::RadioFrame {
                    node: messages_prost::common::Node::Phoenix.into(),
                    payload: Some(messages_prost::radio::radio_frame::Payload::ArgusStrain(
                        messages_prost::sensor::argus::Strain { sensor_id, strain },
                    )),
                    millis_since_start: Instant::now().as_millis(),
                };
                msg.encode_length_delimited(&mut buf.as_mut())
                    .expect("Failed to encode SBG GPS Position");

                SD_CHANNEL.send(("strain.txt", buf)).await;
                sensor_id += 1;
                // update the sensor_id, this is fugly
                match sensor_id {
                    0 => {
                        adc.write_register(
                            Register::INPMUX,
                            register_data::INPMUX_AIN0_POS | register_data::INPMUX_AIN1_NEG,
                        );
                    }
                    1 => {
                        adc.write_register(
                            Register::INPMUX,
                            register_data::INPMUX_AIN2_POS | register_data::INPMUX_AIN3_NEG,
                        );
                    }
                    2 => {
                        adc.write_register(
                            Register::INPMUX,
                            register_data::INPMUX_AIN4_POS | register_data::INPMUX_AIN5_NEG,
                        );
                    }
                    3 => {
                        adc.write_register(
                            Register::INPMUX,
                            register_data::INPMUX_AIN6_POS | register_data::INPMUX_AIN7_NEG,
                        );
                    }
                    4 => {
                        adc.write_register(
                            Register::INPMUX,
                            register_data::INPMUX_AIN8_POS | register_data::INPMUX_AIN9_NEG,
                        );
                    }
                    _ => {
                        sensor_id = 0;
                        adc.write_register(
                            Register::INPMUX,
                            register_data::INPMUX_AIN0_POS | register_data::INPMUX_AIN1_NEG,
                        );
                    }
                }
            }
        } else {
            info!("Failed to read ADC2 data.");
        }
    }
}

// =================================================================================
// Main Entry Point
// =================================================================================

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("System starting...");
    {
        use core::mem::MaybeUninit;
        const HEAP_SIZE: usize = 40000;
        static mut HEAP_MEM: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];
        unsafe { HEAP.init(HEAP_MEM.as_ptr() as usize, HEAP_SIZE) }
    }

    let mut config = embassy_stm32::Config::default();

    {
        use embassy_stm32::rcc::*;
        config.rcc.hsi = Some(HSIPrescaler::DIV1);
        // config.rcc.hse = Some(Hse { freq: Hertz::hz(48_000_000), mode: HseMode::Oscillator });
        // config.rcc.mux.fdcsansel = rcc::mux::Fdcansel::HSE;
        config.rcc.csi = true;
        config.rcc.pll1 = Some(Pll {
            source: PllSource::HSI,
            prediv: PllPreDiv::DIV4,
            mul: PllMul::MUL50,
            divp: Some(PllDiv::DIV2),
            divq: Some(PllDiv::DIV8), // used by SPI3. 100Mhz.
            divr: None,
        });
        config.rcc.pll2 = Some(Pll {
            source: PllSource::HSI,
            prediv: PllPreDiv::DIV4,
            mul: PllMul::MUL50,
            divp: Some(PllDiv::DIV2),
            divq: Some(PllDiv::DIV8), // used by SPI3. 100Mhz.
            divr: None,
        });
        config.rcc.sys = Sysclk::PLL1_P; // 400 Mhz
        config.rcc.ahb_pre = AHBPrescaler::DIV2; // 200 Mhz
        config.rcc.apb1_pre = APBPrescaler::DIV2; // 100 Mhz
        config.rcc.apb2_pre = APBPrescaler::DIV2; // 100 Mhz
        config.rcc.apb3_pre = APBPrescaler::DIV2; // 100 Mhz
        config.rcc.apb4_pre = APBPrescaler::DIV2; // 100 Mhz
        config.rcc.voltage_scale = VoltageScale::Scale1;
    }
    // config.rcc.ls = rcc::LsConfig::default_lse();
    let p = embassy_stm32::init(config);

    // --- Sd Setup ---
    let sd_card: embedded_sdmmc::SdCard<
        embedded_hal_bus::spi::RefCellDevice<
            'static,
            Spi<'static, embassy_stm32::mode::Blocking>,
            Output<'static>,
            Delay,
        >,
        Delay,
    > = sd::setup_sdmmc_interface(p.SPI1, p.PA5, p.PA7, p.PA6, p.PC4);

    let state_machine = StateMachine::new(state_machine::Context {});

    // --- CAN Setup ---
    // let mut can = can::CanConfigurator::new(p.FDCAN2, p.PB12, p.PB13, resources::Irqs);

    // can.set_bitrate(250_000);

    // let mut can = can.into_normal_mode();

    // --- ADS 126 Setup ---
    let mut adc_spi_config = SpiConfig::default();
    adc_spi_config.frequency = mhz(8);
    adc_spi_config.mode = embassy_stm32::spi::MODE_1;

    let adc_spi_bus = Spi::new_blocking(p.SPI4, p.PE2, p.PE6, p.PE5, adc_spi_config);

    let adc1_cs = Output::new(p.PE1, Level::High, Speed::Low);
    let adc2_cs = Output::new(p.PB8, Level::High, Speed::Low);
    let adc1_rst = Output::new(p.PE0, Level::High, Speed::Low);
    let adc2_rst = Output::new(p.PB7, Level::High, Speed::Low);
    let adc1_drdy = ExtiInput::new(p.PB9, p.EXTI9, Pull::Up);
    let adc2_drdy = ExtiInput::new(p.PB6, p.EXTI6, Pull::Up);

    let spi_bus_ref = ADC_SPI_BUS_CELL.init(RefCell::new(adc_spi_bus));

    // let spi_bus_mutex = NoopMutex::new(spi_bus_ref);

    let adc1_spi_device = RefCellDevice::new(spi_bus_ref, adc1_cs, Delay).unwrap();
    let mut adc1 = Ads1262::new(adc1_spi_device, adc1_rst, adc1_drdy);

    let adc2_spi_device = RefCellDevice::new(spi_bus_ref, adc2_cs, Delay).unwrap();
    let mut adc2 = Ads1262::new(adc2_spi_device, adc2_rst, adc2_drdy);
    info!("ADC1 and ADC2 initialized.");

    adc1.reset(&mut Delay).unwrap();
    adc2.reset(&mut Delay).unwrap();
    info!("ADC1 and ADC2 reset.");

    // =========================================================================
    // LOAD CALIBRATION VALUES (Normal Operation)
    // =========================================================================
    #[cfg(not(feature = "calibration"))]
    {
        // IMPORTANT: Manually enter the calibration values obtained from the
        // calibration suite here.
        const ADC1_OFCAL: u32 = 0x000000; // Replace with your value
        const ADC1_FSCAL: u32 = 0x400000; // Replace with your value
        const ADC2_OFCAL: u32 = 0x000000; // Replace with your value
        const ADC2_FSCAL: u32 = 0x400000; // Replace with your value

        info!("Loading production calibration values...");
        // Write the calibration values to the ADC registers
        let adc1_ofcal_bytes = ADC1_OFCAL.to_be_bytes();
        adc1.write_registers(
            Register::OFCAL0,
            &[
                adc1_ofcal_bytes[1],
                adc1_ofcal_bytes[2],
                adc1_ofcal_bytes[3],
            ],
        )
        .unwrap();

        let adc1_fscal_bytes = ADC1_FSCAL.to_be_bytes();
        adc1.write_registers(
            Register::FSCAL0,
            &[
                adc1_fscal_bytes[1],
                adc1_fscal_bytes[2],
                adc1_fscal_bytes[3],
            ],
        )
        .unwrap();

        let adc2_ofcal_bytes = ADC2_OFCAL.to_be_bytes();
        adc2.write_registers(
            Register::OFCAL0,
            &[
                adc2_ofcal_bytes[1],
                adc2_ofcal_bytes[2],
                adc2_ofcal_bytes[3],
            ],
        )
        .unwrap();

        let adc2_fscal_bytes = ADC2_FSCAL.to_be_bytes();
        adc2.write_registers(
            Register::FSCAL0,
            &[
                adc2_fscal_bytes[1],
                adc2_fscal_bytes[2],
                adc2_fscal_bytes[3],
            ],
        )
        .unwrap();

        info!(
            "ADC1 OFCAL: {:#08x}, FSCAL: {:#08x}",
            ADC1_OFCAL, ADC1_FSCAL
        );
        info!(
            "ADC2 OFCAL: {:#08x}, FSCAL: {:#08x}",
            ADC2_OFCAL, ADC2_FSCAL
        );
    }

    #[cfg(feature = "temperature")]
    {
        configure_adc_for_thermocouple(&mut adc1).unwrap();
        info!("ADC1 configured for Thermocouple measurement.");
        configure_adc_for_thermocouple(&mut adc2).unwrap();
    }

    #[cfg(feature = "pressure")]
    {
        configure_adc_for_pressure_sensor(&mut adc1).unwrap();
        info!("ADC1 configured for Pressure Sensor measurement.");
        configure_adc_for_pressure_sensor(&mut adc2).unwrap();
    }

    #[cfg(feature = "strain")]
    {
        configure_adc_for_strain_gauge(&mut adc1).unwrap();
        info!("ADC1 configured for Strain Gauge measurement.");
        configure_adc_for_strain_gauge(&mut adc2).unwrap();
    }

    // When running calibration, we still need a base configuration.
    // We'll use the thermocouple config as a default.
    #[cfg(feature = "calibration")]
    {
        configure_adc_for_thermocouple(&mut adc1).unwrap();
        configure_adc_for_thermocouple(&mut adc2).unwrap();
        info!("ADCs configured for calibration mode.");
    }

    // Delay for the internal reference to settle if it was enabled.
    // This is required for thermocouple mode.
    #[cfg(any(feature = "temperature", feature = "calibration"))]
    {
        Timer::after_millis(20).await;
        info!("Waited for internal reference to settle.");
    }

    // In normal mode, start ADC conversions. In calibration mode, the
    // calibration task will control the ADC.
    #[cfg(not(feature = "calibration"))]
    {
        adc1.send_command(ads::Command::START1).unwrap();
        adc2.send_command(ads::Command::START1).unwrap();
    }

    // --- Spawn Tasks ---

    // Spawn the appropriate tasks based on the feature flags.
    // The calibration task is mutually exclusive with the normal adc tasks.
    #[cfg(feature = "calibration")]
    {
        spawner.must_spawn(calibration::calibration_task(adc1, adc2));
    }

    #[cfg(not(feature = "calibration"))]
    {
        // spawner.must_spawn(temperature_regulator(Adc::new(p.ADC1), p.PB1, Output::new(p.PE11, Level::Low, Speed::Low)));
        spawner.must_spawn(adc1_task(adc1));
        spawner.must_spawn(adc2_task(adc2));
        spawner.must_spawn(sd::sdmmc_task(sd_card));
        spawner.must_spawn(sm_task(spawner, state_machine));
    }
}
