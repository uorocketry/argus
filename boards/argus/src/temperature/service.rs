use core::str::FromStr;

use defmt::info;
use embassy_time::Instant;
use heapless::LinearMap;

use crate::adc::config::ADC_COUNT;
use crate::adc::driver::types::{AnalogChannel, DataRate, Filter, Gain, ReferenceRange};
use crate::adc::service::AdcService;
use crate::adc::types::AdcDevice;
use crate::sd::csv::types::SerializeCSV;
use crate::sd::service::SDCardService;
use crate::sd::types::{FileName, OperationScope, SdCardError};
use crate::serial::service::SerialService;
use crate::temperature::config::{CHANNEL_COUNT, LINEAR_TRANSFORMATIONS_FILE_NAME, RTD_RESISTANCE_AT_0C};
use crate::temperature::rtd;
use crate::temperature::thermocouple::type_k;
use crate::temperature::types::{LinearTransformation, TemperatureServiceError, ThermocoupleChannel, ThermocoupleReading, ThermocoupleReadingQueue};
use crate::utils::types::AsyncMutex;

// A channel for buffering the temperature readings and decoupling the logging to sd task from the measurement task
pub static THERMOCOUPLE_READING_QUEUE: ThermocoupleReadingQueue = ThermocoupleReadingQueue::new();

pub struct TemperatureService {
	// Other services are passed by a mutex to ensure safe concurrent access
	pub adc_service: &'static AsyncMutex<AdcService>,
	pub sd_card_service: &'static AsyncMutex<SDCardService>,
	pub serial_service: &'static AsyncMutex<SerialService>,

	// Store the last RTD reading in Celsius to use for cold junction compensation
	// This is cached here to avoid reading the RTD multiple times when reading multiple thermocouples
	// We have one RTD per ADC, so we store an array of last readings
	pub last_rtd_reading: [Option<f32>; ADC_COUNT],

	// Linear transformations that are applied on top of the raw readings for each ADC and channel
	pub transformations: LinearMap<AdcDevice, LinearMap<ThermocoupleChannel, LinearTransformation, CHANNEL_COUNT>, ADC_COUNT>,
}

impl TemperatureService {
	pub fn new(
		adc_service: &'static AsyncMutex<AdcService>,
		sd_card_service: &'static AsyncMutex<SDCardService>,
		serial_service: &'static AsyncMutex<SerialService>,
	) -> Self {
		Self {
			adc_service,
			sd_card_service,
			serial_service,
			last_rtd_reading: [None; ADC_COUNT],
			transformations: LinearMap::new(),
		}
	}

	pub async fn setup(&mut self) -> Result<(), TemperatureServiceError> {
		for driver in self.adc_service.lock().await.drivers.iter_mut() {
			driver.reference_range = ReferenceRange::Avdd;
			driver.data_rate = DataRate::Sps100;
			driver.filter = Filter::Sinc3;
			driver.enable_internal_reference = true;
			driver.gain = Gain::G32;
			driver.delay_after_setting_channel = 50; // 50 ms delay to allow the ADC to stabilize after switching channels
			driver.apply_configurations().await?;
		}

		self.load_transformations().await?;
		Ok(())
	}

	pub async fn read_thermocouple(
		&mut self,
		adc: AdcDevice,
		channel: ThermocoupleChannel,
	) -> Result<ThermocoupleReading, TemperatureServiceError> {
		let mut adc_service = self.adc_service.lock().await;

		// Get the respective "adc channel" pair for the "thermocouple channel"
		let (positive_channel, negative_channel) = channel.to_analog_input_channel_pair();

		// Read the voltage from the ADC in millivolts
		let voltage = adc_service.drivers[adc as usize]
			.read_differential(positive_channel, negative_channel)
			.await? * 1000.0; // Convert to millivolts

		// Get the cold junction temperature from the last RTD reading for this ADC
		let cold_junction_temperature = self.last_rtd_reading[adc as usize];

		let thermocouple_reading = ThermocoupleReading {
			timestamp: Instant::now().as_millis(),
			voltage,
			// SHOULD DO: remove the uncompensated temperature field once everything is confirmed working after testing
			uncompensated_temperature: type_k::convert_voltage_to_temperature(voltage as f64)?,
			compensated_temperature: type_k::convert_voltage_to_temperature_with_cold_junction_compensation(
				voltage as f64,
				cold_junction_temperature.unwrap_or(0.0) as f64,
			)?,
			cold_junction_temperature: cold_junction_temperature.unwrap_or(0.0),
		};

		Ok(thermocouple_reading)
	}

	pub async fn read_rtd(
		&mut self,
		adc: AdcDevice,
	) -> Result<f32, TemperatureServiceError> {
		let mut adc_service = self.adc_service.lock().await;

		// Note: This is based on Argus V2 design as of September 22, 2025
		// The AIN8-9 sequence is flipped accidentally so AIN9 is before the RTD and AIN8 is after the RTD
		let voltage_before_rtd = adc_service.drivers[adc as usize].read_single_ended(AnalogChannel::AIN9).await?;
		let voltage_after_rtd = adc_service.drivers[adc as usize].read_single_ended(AnalogChannel::AIN8).await?;

		// I = voltage_after_rtd / R6
		// measured_resistance = V_RTD / I = R6 * (voltage_before_rtd - voltage_after_rtd) / voltage_after_rtd
		const R6: f32 = 1000.0;
		let measured_resistance = R6 * (voltage_before_rtd - voltage_after_rtd) / voltage_after_rtd;
		let estimated_temperature = rtd::convert_resistance_to_temperature(RTD_RESISTANCE_AT_0C, measured_resistance);
		Ok(estimated_temperature)
	}

	pub async fn load_transformations(&mut self) -> Result<(), TemperatureServiceError> {
		let result = self.sd_card_service.lock().await.read(
			OperationScope::Root,
			FileName::from_str(LINEAR_TRANSFORMATIONS_FILE_NAME).unwrap(),
			|line| {
				if *line == LinearTransformation::get_csv_header() {
					return true; // Skip header line
				}
				let transformation = LinearTransformation::from_csv_line(line);
				self.load_transformation(transformation);
				true // Continue reading
			},
		);

		match result {
			Ok(_) => (),
			Err(SdCardError::NotFound) => {
				// If transformations not found, keep using the defaults and ignore this error.
				info!("Linear transformations file not found, using defaults. Gain = 1, Offset = 0");
			}
			Err(e) => return Err(TemperatureServiceError::SdCardError(e)),
		}
		Ok(())
	}

	pub fn load_transformation(
		&mut self,
		transformation: LinearTransformation,
	) {
		if !self.transformations.contains_key(&transformation.adc) {
			let _ = self.transformations.insert(transformation.adc, LinearMap::new());
		}
		let map = self.transformations.get_mut(&transformation.adc).unwrap();
		let _ = map.insert(transformation.channel, transformation);
	}
}
