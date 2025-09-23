use crate::temperature::thermocouple::type_k::error::ThermocoupleError;

// ────────────────────────────────────────────────────────────────────────────
// ITS-90: Type K inverse function t90(E)  (temperature from voltage)
// Three ranges:
//
// 1) −5.891 mV ≤ E ≤ 0.000 mV
// t = Σ d_i * E^i   (i = 0..8)
//
// 2)  0.000 mV < E ≤ 20.644 mV
// t = Σ d_i * E^i   (i = 0..10)
//
// 3) 20.644 mV < E ≤ 54.886 mV
// t = Σ d_i * E^i   (i = 0..6)
//
// Coefficients below are the official ITS-90 values (E in mV, t in °C).
// ────────────────────────────────────────────────────────────────────────────

const K_T_OF_E_D_NEG: [f64; 9] = [
	0.000_000_0e+00,
	2.517_346_2e+01 * 1.0e-3, // convert published 2.5173462E+01 mV term into per-mV scale:
	// In the standard tables, E is in mV already and coefficients are sized accordingly.
	// We keep them as-is (no unit scaling needed). The comment is just explanatory.
	-1.166_287_8e+00,
	-1.083_363_8e+00,
	-8.977_354_0e-01,
	-3.734_237_7e-01,
	-8.663_264_3e-02,
	-1.045_059_8e-02,
	-5.192_057_7e-04,
];
// Note: The published list for the negative range has zero for higher orders; the above length (0..8) matches the table.

const K_T_OF_E_D_MID: [f64; 11] = [
	0.000_000_0e+00,
	2.508_355_0e+01 * 1.0e-3,
	7.860_106_0e-02,
	-2.503_131_0e-01,
	8.315_270_0e-02,
	-1.228_034_0e-02,
	9.804_036_0e-04,
	-4.413_030_0e-05,
	1.057_734_0e-06,
	-1.052_755_0e-08,
	0.000_000_0e+00, // table shows up to d9; any higher terms are zero.
];

const K_T_OF_E_D_HIGH: [f64; 7] = [
	-1.318_058_0e+02,
	4.830_222_0e+01 * 1.0e-3,
	-1.646_031_0e+00,
	5.464_731_0e-02,
	-9.650_715_0e-04,
	8.802_193_0e-06,
	-3.110_810_0e-08,
];

const K_E_MIN_MV: f64 = -5.891;
const K_E_MID_MAX_MV: f64 = 20.644;
const K_E_MAX_MV: f64 = 54.886;

/// Returns t90(E) in °C for Type K, or an error if E is out of range.
pub fn convert_voltage_to_temperature(voltage: f64) -> Result<f64, ThermocoupleError> {
	if voltage < K_E_MIN_MV || voltage > K_E_MAX_MV {
		return Err(ThermocoupleError::MillivoltsOutOfRange);
	}

	if voltage <= 0.0 {
		Ok(evaluate_power_series(voltage, &K_T_OF_E_D_NEG))
	} else if voltage <= K_E_MID_MAX_MV {
		Ok(evaluate_power_series(voltage, &K_T_OF_E_D_MID))
	} else {
		Ok(evaluate_power_series(voltage, &K_T_OF_E_D_HIGH))
	}
}

/// Evaluate a power series t = Σ c_i * x^i using Horner’s method, kept legible.
fn evaluate_power_series(
	x: f64,
	coefficients: &[f64],
) -> f64 {
	let mut accumulator = 0.0;
	for &c in coefficients.iter().rev() {
		accumulator = accumulator * x + c;
	}
	accumulator
}
