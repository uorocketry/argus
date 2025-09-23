#![feature(impl_trait_in_assoc_type)]
#![no_std]
#![no_main]

#[cfg(test)]
#[embedded_test::tests]
mod tests {
	// SHOULD DO: embedded_test's panic handler is crashing with panic_probe so we have to use embedded_tests' panic handler
	// But that makes panics not "deformat" properly so we can't see the panic message
	// But with assert_eq, etc we don't need to see panic messages anyway
	// So for now this is acceptable, but we should try to fix this in the future

	use core::str::FromStr;

	use argus::sd::service::SDCardService;
	use argus::sd::types::{FileName, Line, OperationScope};
	use argus::utils::hal::configure_hal;
	use defmt_rtt as _;
	use heapless::String;

	#[init]
	fn init() -> SDCardService {
		let peripherals = configure_hal();
		let sd_card_service = SDCardService::new(peripherals.SPI1, peripherals.PA5, peripherals.PA7, peripherals.PA6, peripherals.PC4);
		sd_card_service
	}

	#[test]
	fn writing_directly_to_sd_card(mut sd_card_service: SDCardService) {
		let path: String<12> = FileName::from_str("test.txt").unwrap();
		let text = Line::from_str("Hello, world!").unwrap();
		sd_card_service.delete(OperationScope::Root, path.clone()).unwrap();
		sd_card_service.write(OperationScope::Root, path.clone(), text.clone()).unwrap();
		sd_card_service
			.read(OperationScope::Root, path.clone(), |line| {
				assert_eq!(line.as_str(), text.as_str());
				return false;
			})
			.unwrap();

		let lines = sd_card_service.read_fixed_number_of_lines::<2>(OperationScope::Root, path).unwrap();
		assert_eq!(lines.len(), 1); // Even through read 2 lines, only 1 line exists
		assert_eq!(lines[0].as_str(), text.as_str());
	}
}
