pub mod error;
pub mod forward;
pub mod inverse;

pub use error::*;
pub use forward::*;
pub use inverse::*;

pub fn convert_voltage_to_temperature_with_cold_junction_compensation(
	measured_voltage: f64,
	cold_junction_temperature: f64,
) -> Result<f64, ThermocoupleError> {
	let cj_mv = convert_temperature_to_voltage(cold_junction_temperature).ok_or(ThermocoupleError::ColdJunctionTemperatureOutOfRange)?;
	let compensated_mv = measured_voltage + cj_mv;
	convert_voltage_to_temperature(compensated_mv)
}
