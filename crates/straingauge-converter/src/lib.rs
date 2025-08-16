#![no_std]
//!
//! This crate contains code to convert strain gauge voltages to temperatures.
//!

/// Coefficients for strain gauge conversion
const GAUGE_FACTOR_DEFAULT: f64 = 2.0;
const V_REF: f64 = 5.0; // Output voltage reference

/*
* Function to convert voltage to strain in quarter bridge configuration
*
* @param voltage: f64 - The voltage output from the strain gauge
* @param gauge_factor: f64 - The gauge factor of the strain gauge default is 2.0
* @return strain: f64 - The calculated strain value
 */
pub fn voltage_to_strain_quart(voltage: f64, gauge_factor: f64) -> f64 {
    let mut strain = (voltage) / ((0.25) * GAUGE_FACTOR_DEFAULT * V_REF);
    // Apply correction factor
    if gauge_factor != 0.0 || !gauge_factor.is_nan() {
        strain = strain * (GAUGE_FACTOR_DEFAULT / gauge_factor);
    }
    return strain;
}

/*
* Function to convert voltage to strain in half bridge configuration
*
* @param voltage: f64 - The voltage output from the strain gauge
* @param gauge_factor: f64 - The gauge factor of the strain gauge default is 2.0
* @return strain: f64 - The calculated strain value
 */
pub fn voltage_to_strain_half(voltage: f64, gauge_factor: f64) -> f64 {
    let mut strain = (voltage) / ((0.5) * GAUGE_FACTOR_DEFAULT * V_REF);
    // Apply correction factor
    if gauge_factor != 0.0 || !gauge_factor.is_nan() {
        strain = strain * (GAUGE_FACTOR_DEFAULT / gauge_factor);
    }
    return strain;
}

/*
* Function to convert voltage to strain in full bridge configuration.
*
* @param voltage: f64 - The voltage output from the strain gauge
* @param gauge_factor: f64 - The gauge factor of the strain gauge
* @return strain: f64 - The calculated strain value
 */
pub fn voltage_to_strain_full(voltage: f64, gauge_factor: f64) -> f64 {
    let mut strain = (voltage) / (GAUGE_FACTOR_DEFAULT * V_REF);
    // Apply correction factor
    if gauge_factor != 0.0 || !gauge_factor.is_nan() {
        strain = strain * (GAUGE_FACTOR_DEFAULT / gauge_factor);
    }
    return strain;
}
