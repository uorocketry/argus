use libm::sqrtf;

// For RTDs (PT100, PT1000, etc).
// Reference: IEC 60751:2016
// Uses the callandar-van dusen equation to convert resistance to temperature.
// Valid for -200 °C to +850 °C
// See https://www.ti.com/lit/an/sbaa275a/sbaa275a.pdf?ts=1758561034572
pub fn convert_resistance_to_temperature(
	resistance_at_0c: f32,
	measured_resistance: f32,
) -> f32 {
	// IEC 60751 constants for α = 0.00385
	const A: f32 = 3.9083e-3;
	const B: f32 = -5.775e-7;
	const C: f32 = -4.183e-12; // used only below 0 °C

	// Normalize measured resistance to ratio against R0.
	// This keeps numbers close to 1.0 and improves numeric behavior.
	let resistance_ratio = measured_resistance / resistance_at_0c;

	// ----- Case 1: Temperature ≥ 0 °C -----
	// Quadratic model: R(T) = R0 * (1 + A*T + B*T^2)
	// Inverse: T = (-A + sqrt(A^2 - 4*B*(1 - R/R0))) / (2*B)
	// Use only if the discriminant is non-negative and the result is ≥ 0.
	let discriminant: f32 = A * A - 4.0 * B * (1.0 - resistance_ratio);

	if discriminant >= 0.0 {
		let temperature_celsius = (-A + sqrtf(discriminant)) / (2.0 * B); // B < 0

		if temperature_celsius >= 0.0 && temperature_celsius.is_finite() {
			return temperature_celsius;
		}
	}

	// ----- Case 2: Temperature < 0 °C -----
	// Full Callendar–Van Dusen (includes coefficient C):
	// R(T) = R0 * (1 + A*T + B*T^2 + C*(T - 100)*T^3)
	// Solve numerically with Newton–Raphson.

	// Initial guess: linearized around 0 °C (good for small negative temps).
	let mut temperature_estimate = (resistance_ratio - 1.0) / A;

	// Perform a small, fixed number of iterations similar to gradient descent
	// for <0 °C over the usual range. Stops early if the correction is tiny.
	for _ in 0..8 {
		let t = temperature_estimate;
		let t2 = t * t;
		let t3 = t2 * t;

		// Predicted resistance from current temperature estimate
		let predicted_measured_resistance = resistance_at_0c * (1.0 + A * t + B * t2 + C * (t - 100.0) * t3);

		// Error relative to the measured resistance
		let resistance_error = predicted_measured_resistance - measured_resistance;

		// Derivative dR/dT (expanded; avoids powi)
		// d/dT [R0*(1 + A*T + B*T^2 + C*(T - 100)*T^3)]
		//   = R0*(A + 2*B*T + C*(4*T^3 - 300*T^2))
		let derivative_ohms_per_c = resistance_at_0c * (A + 2.0 * B * t + C * (4.0 * t3 - 300.0 * t2));

		// Guard against pathological derivative values
		if derivative_ohms_per_c == 0.0 || !derivative_ohms_per_c.is_finite() {
			break;
		}

		let correction_step_celsius = resistance_error / derivative_ohms_per_c;
		temperature_estimate -= correction_step_celsius;

		if correction_step_celsius.abs() < 1e-4 {
			break;
		}
	}

	temperature_estimate
}
