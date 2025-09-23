// ────────────────────────────────────────────────────────────────────────────
// ITS-90: Type K reference function E(t)  (voltage from temperature)
// Two ranges:
//
// A) -270 °C ≤ t ≤ 0 °C
// E = Σ c_i * t^i  (i = 0..10)
//
// B) 0 °C < t ≤ 1372 °C
// E = Σ c_i * t^i  (i = 0..9)  +  a0 * exp( a1 * (t - a2)^2 )
//
// Coefficients are from the official tables (units: E in mV, t in °C).
// ────────────────────────────────────────────────────────────────────────────

const K_E_OF_T_NEG_COEFFS: [f64; 11] = [
	0.000_000_000_000e+00,
	0.394_501_280_250e-01,
	0.236_223_735_980e-04,
	-0.328_589_067_840e-06,
	-0.499_048_287_770e-08,
	-0.675_090_591_730e-10,
	-0.574_103_274_280e-12,
	-0.310_888_728_940e-14,
	-0.104_516_093_650e-16,
	-0.198_892_668_780e-19,
	-0.163_226_974_860e-22,
];

const K_E_OF_T_POS_COEFFS: [f64; 10] = [
	-0.176_004_136_860e-01,
	0.389_212_049_750e-01,
	0.185_587_700_320e-04,
	-0.994_575_928_740e-07,
	0.318_409_457_190e-09,
	-0.560_728_448_890e-12,
	0.560_750_590_590e-15,
	-0.320_207_200_030e-18,
	0.971_511_471_520e-22,
	-0.121_047_212_750e-25,
];

const K_E_OF_T_POS_A0: f64 = 0.118_597_600_000e+00;
const K_E_OF_T_POS_A1: f64 = -0.118_343_200_000e-03;
const K_E_OF_T_POS_A2: f64 = 0.126_968_600_000e+03;

/// Returns E(t) in voltage for a Type K thermocouple, or None if t is out of range.
pub fn convert_temperature_to_voltage(temperature: f64) -> Option<f64> {
	if temperature < -270.0 || temperature > 1372.0 {
		return None;
	}

	// Evaluate polynomial with Horner’s method (kept readable).
	let polynomial = |coefficients: &[f64]| -> f64 {
		let mut accumulator = 0.0;
		// Highest order first for Horner’s method:
		for &coefficient in coefficients.iter().rev() {
			accumulator = accumulator * temperature + coefficient;
		}
		accumulator
	};

	if temperature <= 0.0 {
		Some(polynomial(&K_E_OF_T_NEG_COEFFS))
	} else {
		// Above 0 °C we add the exponential correction term.
		let base = polynomial(&K_E_OF_T_POS_COEFFS);
		let dt = temperature - K_E_OF_T_POS_A2;
		let exp_term = K_E_OF_T_POS_A0 * libm::exp(K_E_OF_T_POS_A1 * (dt * dt));
		Some(base + exp_term)
	}
}
