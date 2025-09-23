use core::marker::PhantomData;

use embedded_sdmmc::{TimeSource, Timestamp};

/// Fake time source for the SD card.
pub struct FakeTimeSource {
	_marker: PhantomData<*const ()>,
}

impl Default for FakeTimeSource {
	fn default() -> Self {
		Self::new()
	}
}

impl FakeTimeSource {
	pub fn new() -> Self {
		FakeTimeSource { _marker: PhantomData }
	}
}

impl TimeSource for FakeTimeSource {
	fn get_timestamp(&self) -> Timestamp {
		// SHOULD DO: replace with an actual time source like RTC
		Timestamp {
			year_since_1970: 55,   // 2025
			zero_indexed_month: 0, // Jan
			zero_indexed_day: 0,   // 1st
			hours: 0,
			minutes: 0,
			seconds: 0,
		}
	}
}
