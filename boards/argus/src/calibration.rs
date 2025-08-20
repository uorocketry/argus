//! # ADC Calibration Suite
//!
//! This module provides a task for calibrating the ADS1262 ADCs.
//! It is enabled by the `calibration` feature flag.

use defmt::*;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::Output;
use embassy_stm32::mode::Blocking;
use embassy_stm32::spi::Spi;
use embassy_time::{Delay, Timer};
use embedded_hal_bus::spi::RefCellDevice;

use crate::ads::{Ads1262, Command, Register};

type AdcType<'a> =
    Ads1262<RefCellDevice<'a, Spi<'a, Blocking>, Output<'a>, Delay>, Output<'a>, ExtiInput<'a>>;

/// Runs an interactive calibration sequence for both ADCs.
///
/// This task will guide the user through offset and gain calibration
/// via log messages and then print the resulting calibration values.
#[embassy_executor::task]
pub async fn calibration_task(mut adc1: AdcType<'static>, mut adc2: AdcType<'static>) {
    info!("\n\n--- ADC Calibration Suite Initialized ---\n");

    // --- Step 1: Offset Calibration ---
    info!("--> Step 1: Offset (Zero) Calibration");
    info!("Please connect sensors and apply ZERO load/pressure, or a 0°C reference.");
    info!("Offset calibration will begin in 10 seconds...");
    Timer::after_secs(10).await;

    info!("Performing system offset calibration (SYOCAL)...");
    adc1.send_command(Command::SYOCAL1).unwrap();
    adc2.send_command(Command::SYOCAL1).unwrap();
    // The datasheet indicates that calibration can take up to 819 ms.
    // We'll wait a full second to be safe.
    Timer::after_secs(1).await;
    info!("Offset calibration complete.");

    // --- Step 2: Gain Calibration ---
    info!("\n--> Step 2: Gain (Full-Scale) Calibration");
    info!("Please apply a known FULL-SCALE load/pressure, or a high temp reference.");
    info!("Gain calibration will begin in 10 seconds...");
    Timer::after_secs(10).await;

    info!("Performing system gain calibration (SYGCAL)...");
    adc1.send_command(Command::SYGCAL1).unwrap();
    adc2.send_command(Command::SYGCAL1).unwrap();
    Timer::after_secs(1).await;
    info!("Gain calibration complete.");

    // --- Step 3: Read and Display Results ---
    info!("\n--> Step 3: Reading Calibration Registers");

    let mut buf = [0u8; 3];

    // Read ADC1 OFCAL
    adc1.read_registers(Register::OFCAL0, 3, &mut buf).unwrap();
    let adc1_ofcal = u32::from_be_bytes([0, buf[0], buf[1], buf[2]]);

    // Read ADC1 FSCAL
    adc1.read_registers(Register::FSCAL0, 3, &mut buf).unwrap();
    let adc1_fscal = u32::from_be_bytes([0, buf[0], buf[1], buf[2]]);

    // Read ADC2 OFCAL
    adc2.read_registers(Register::OFCAL0, 3, &mut buf).unwrap();
    let adc2_ofcal = u32::from_be_bytes([0, buf[0], buf[1], buf[2]]);

    // Read ADC2 FSCAL
    adc2.read_registers(Register::FSCAL0, 3, &mut buf).unwrap();
    let adc2_fscal = u32::from_be_bytes([0, buf[0], buf[1], buf[2]]);

    info!("\n--- CALIBRATION COMPLETE ---");
    info!("Please copy these values into your main.rs file.");
    info!("ADC1 OFCAL: {:#08x}", adc1_ofcal);
    info!("ADC1 FSCAL: {:#08x}", adc1_fscal);
    info!("ADC2 OFCAL: {:#08x}", adc2_ofcal);
    info!("ADC2 FSCAL: {:#08x}", adc2_fscal);
    info!("\nCalibration suite finished. The device will now idle.");
}
