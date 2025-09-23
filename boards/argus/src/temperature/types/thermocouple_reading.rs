use core::str::FromStr;

use defmt::Format;
use serde::{Deserialize, Serialize};

use crate::sd::csv::types::SerializeCSV;
use crate::sd::types::Line;

// Represents a single temperature reading from a thermocouple channel
#[derive(Debug, Clone, Copy, Format, Serialize, Deserialize)]
pub struct ThermocoupleReading {
	// Timestamp of the reading in milliseconds since epoch
	pub timestamp: u64,

	// Thermocouple voltage difference measured in millivolts
	pub voltage: f32,

	// Cold-junction-compensated temperature of the thermocouple in degrees Celsius
	pub compensated_temperature: f64,

	// Uncompensated temperature of the thermocouple in degrees Celsius
	pub uncompensated_temperature: f64,

	// Temperature of the cold junction in degrees Celsius
	pub cold_junction_temperature: f32,
}

impl SerializeCSV for ThermocoupleReading {
	fn get_csv_header() -> Line {
		Line::from_str(
			"Timestamp (ms),\
			Voltage (mV),\
			Compensated Temperature (C),\
			Uncompensated Temperature (C),\
			Cold Junction Temperature (C)\n",
		)
		.unwrap()
	}
}
