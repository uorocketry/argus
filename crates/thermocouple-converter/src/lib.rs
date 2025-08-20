//!
//! This crate contains code to convert type K thermocouple voltages to temperatures.
//!

#![no_std]

/// Type K thermocouple coefficients for polynomial voltage to temperature conversion.
/// See https://www.eevblog.com/forum/metrology/a-dive-into-k-type-thermocouple-maths/
pub const TYPE_K_COEF: [[f64; 10]; 3] = [
    [
        // Coefficients for -5.891 <= voltage <= 0.0
        0.0000000E+00,
        2.5173462E+01,
        -1.1662878E+00,
        -1.0833638E+00,
        -8.9773540E-01,
        -3.7342377E-01,
        -8.6632643E-02,
        -1.0450598E-02,
        -5.1920577E-04,
        0.0000000E+00,
    ],
    [
        // Coefficients for 0.0 <= voltage <= 20.644
        0.000000E+00,
        2.508355E+01,
        7.860106E-02,
        -2.503131E-01,
        8.315270E-02,
        -1.228034E-02,
        9.804036E-04,
        -4.413030E-05,
        1.057734E-06,
        -1.052755E-08,
    ],
    [
        // Coefficients for 20.644 <= voltage <= 54.886
        -1.318058E+02,
        4.830222E+01,
        -1.646031E+00,
        5.464731E-02,
        -9.650715E-04,
        8.802193E-06,
        -3.110810E-08,
        0.000000E+00,
        0.000000E+00,
        0.000000E+00,
    ],
];

/// Converts a 32-bit ADC reading to a temperature in celsius.
pub fn adc_to_celsius(adc_reading: i32) -> Option<f64> {
    voltage_to_celsius(adc_to_voltage(adc_reading))
}

/// Converts a 32-bit ADC reading to a voltage.
pub fn adc_to_voltage(adc_reading: i32) -> f64 {
    const REFERENCE_VOLTAGE: f64 = 5.0;
    const MAX_ADC_VALUE: f64 = 4_294_967_296.0;
    // change 32 to be waht the gain is
    const V_SCALE: f64 = (REFERENCE_VOLTAGE / MAX_ADC_VALUE) / 32.0;

    adc_reading as f64 * V_SCALE
}

/// Converts voltage to celsius for type K thermocouples.
pub fn voltage_to_celsius(mut voltage: f64) -> Option<f64> {
    voltage *= 1000.0;
    return match voltage {
        -5.891..=0.0 => Some(calc_temp_exponential(voltage, &TYPE_K_COEF[0])),
        0.0..=20.644 => Some(calc_temp_exponential(voltage, &TYPE_K_COEF[1])),
        20.644..=54.886 => Some(calc_temp_exponential(voltage, &TYPE_K_COEF[2])),

        // Insane temperature ranges that should never be reached.
        // Hitting this is a strong indicator of a bug in the Argus system.
        _ => None,
    };
}

/// Calculates temperature using the NIST's exponential polynomial.
fn calc_temp_exponential(voltage: f64, coef: &[f64]) -> f64 {
    let mut result = 0.0;
    for k in 0..coef.len() {
        result += coef[k] * pow(voltage, k as i32);
    }
    return result;
}

/// Floating point exponentiation function.
/// Cannot access std::f64::powi in no_std environment.
fn pow(base: f64, exp: i32) -> f64 {
    if exp < 0 {
        return 1.0 / pow(base, -exp);
    }

    let mut result = 1.0;
    for _ in 0..exp {
        result *= base;
    }
    return result;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voltage_to_celsius_converts_expected_ranges() {
        let result: f64 = voltage_to_celsius(20.644/ 1000.0).unwrap();
        assert!(499.97 <= result && 500.0 >= result);

        let result: f64 = voltage_to_celsius(6.138/ 1000.0).unwrap();
        assert!(150.01 <= result && 150.03 >= result);

        let result: f64 = voltage_to_celsius(0.039/ 1000.0).unwrap();
        assert!(0.97 <= result && 0.98 >= result);

        let result: f64 = voltage_to_celsius(-0.778/ 1000.0).unwrap();
        assert!(-20.03 <= result && -20.01 >= result);

        let result: f64 = voltage_to_celsius(10.0/ 1000.0).unwrap();
        assert!(246.1 <= result && 246.3 >= result);
    }

    #[test]
    fn voltage_to_celsius_panics_on_temp_too_cold() {
        assert!(voltage_to_celsius(-6.0).is_none());
    }

    #[test]
    fn voltage_to_celsius_panics_on_temp_too_hot() {
        assert!(voltage_to_celsius(-6.0).is_none());
    }
}
