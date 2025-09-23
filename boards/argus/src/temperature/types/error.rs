use defmt::Format;
use derive_more::From;

use crate::adc::service::AdcError;
use crate::sd::types::SdCardError;
use crate::serial::service::UsartError;
use crate::temperature::thermocouple::type_k::ThermocoupleError;

#[derive(Debug, Format, From)]
pub enum TemperatureServiceError {
	AdcError(AdcError),
	UsartError(UsartError),
	SdCardError(SdCardError),
	ThermocoupleError(ThermocoupleError),
}
