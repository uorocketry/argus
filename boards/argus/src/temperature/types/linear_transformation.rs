use core::str::FromStr;

use defmt::Format;
use serde::{Deserialize, Serialize};

use crate::adc::types::AdcDevice;
use crate::sd::csv::types::SerializeCSV;
use crate::sd::types::Line;
use crate::temperature::types::thermocouple_channel::ThermocoupleChannel;

// Represents a linear transformation applied to the thermocouple readings
// corrected_value = value_with_error * gain + offset
#[derive(Debug, Clone, Copy, Format, Serialize, Deserialize)]
pub struct LinearTransformation {
	pub adc: AdcDevice,
	pub channel: ThermocoupleChannel,
	pub gain: f64,
	pub offset: f64,
}
impl Default for LinearTransformation {
	fn default() -> Self {
		Self {
			adc: AdcDevice::Adc1,
			channel: ThermocoupleChannel::Channel1,
			gain: 1.0,   // Default to unity gain
			offset: 0.0, // Default to zero offset
		}
	}
}
impl SerializeCSV for LinearTransformation {
	fn get_csv_header() -> Line {
		Line::from_str("ADC Index,Channel Index,Gain,Offset\n").unwrap()
	}
}
