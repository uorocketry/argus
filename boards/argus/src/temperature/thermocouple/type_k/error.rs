use defmt::Format;

/// Errors that can occur during conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Format)]
pub enum ThermocoupleError {
	/// The provided millivolt value is outside the supported ITS-90 range for Type K.
	MillivoltsOutOfRange,
	/// The provided cold-junction temperature is outside the supported ITS-90 range for Type K.
	ColdJunctionTemperatureOutOfRange,
}
