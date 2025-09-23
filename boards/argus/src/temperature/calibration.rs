use core::fmt::Debug;
use core::str::FromStr;

use defmt::Format;
use heapless::{format, String, Vec};

use crate::adc::config::ADC_COUNT;
use crate::adc::types::AdcDevice;
use crate::sd::csv::types::SerializeCSV;
use crate::sd::types::{FileName, OperationScope};
use crate::serial::service::UsartError;
use crate::temperature::config::{CHANNEL_COUNT, LINEAR_TRANSFORMATIONS_FILE_NAME, MAX_CALIBRATION_DATA_POINTS};
use crate::temperature::service::TemperatureService;
use crate::temperature::types::{LinearTransformation, TemperatureServiceError, ThermocoupleChannel};

// Calibration logic has been separated into its own file for clarity
impl TemperatureService {
	pub async fn calibrate(&mut self) -> Result<(), TemperatureServiceError> {
		// Prompt for ADC index
		let adc_index: usize = self
			.prompt("Starting temperature calibration. Enter ADC index (Starts from 0):\"\n")
			.await?;
		if adc_index >= ADC_COUNT {
			self.send_message("Invalid ADC index.\n").await?;
			return Ok(());
		}
		let adc = AdcDevice::from(adc_index);

		// Prompt for channel
		let channel_index: usize = self.prompt("Enter thermocouple channel index (Starts from 0):\"\n").await?;
		if channel_index >= CHANNEL_COUNT {
			self.send_message("Invalid channel index.\n").await?;
			return Ok(());
		}
		let channel = ThermocoupleChannel::from(channel_index);

		// Prompt for number of data points
		let data_points_count: u8 = self.prompt("Enter number of data points to use for the linear fit:\n").await?;
		if data_points_count < 2 {
			self.send_message("Minimum 2 data points is required.\n").await?;
			return Ok(());
		}
		if data_points_count > MAX_CALIBRATION_DATA_POINTS as u8 {
			let error_message: String<64> = format!("Too many data points. Maximum is {}.\n", MAX_CALIBRATION_DATA_POINTS).unwrap();
			self.send_message(error_message.as_str()).await?;
			return Ok(());
		}

		// Start collecting data points
		let mut calibration_data_points: Vec<CalibrationDataPoint, MAX_CALIBRATION_DATA_POINTS> = Vec::new();
		for data_point_index in 0..data_points_count {
			let message: String<64> = format!("Data Point #{}. Enter expected value in degrees celsius:\n", data_point_index + 1).unwrap();
			let expected_temperature: f64 = self.prompt(message.as_str()).await?;
			let measured_temperature: f64 = self.read_thermocouple(adc, channel).await?.compensated_temperature;
			let data_point = CalibrationDataPoint {
				expected_temperature,
				measured_temperature,
			};
			calibration_data_points.push(data_point).unwrap(); // Safe due to prior checks

			let confirmation_message: String<64> = format!(
				"Recorded data point: Expected = {:.2} °C, Measured = {:.2} °C\n",
				expected_temperature, measured_temperature
			)
			.unwrap();
			self.send_message(confirmation_message.as_str()).await?;
		}

		// Perform Ordinary Least Squares Fit
		let transformation = self.run_ordinary_least_squares_fit(adc, channel, calibration_data_points);
		let result_message: String<128> = format!(
			"Ordinary Least Squares Fit complete. Gain: {:.6}, Offset: {:.2} °C\n",
			transformation.gain, transformation.offset
		)
		.unwrap();
		self.send_message(result_message.as_str()).await?;

		// Update calibration for the channel
		self.save_transformation(transformation).await?;
		Ok(())
	}

	fn run_ordinary_least_squares_fit(
		&self,
		adc: AdcDevice,
		channel: ThermocoupleChannel,
		data_points: Vec<CalibrationDataPoint, MAX_CALIBRATION_DATA_POINTS>,
	) -> LinearTransformation {
		// Keeping all types as f64
		let data_points_count: f64 = data_points.len() as f64;

		// Accumulate sums
		let mut sum_x: f64 = 0.0; // measured
		let mut sum_y: f64 = 0.0; // expected
		let mut sum_xx: f64 = 0.0; // measured^2
		let mut sum_xy: f64 = 0.0; // measured * expected

		for data_point in data_points.iter() {
			let x = data_point.measured_temperature;
			let y = data_point.expected_temperature;
			sum_x += x;
			sum_y += y;
			sum_xx += x * x;
			sum_xy += x * y;
		}

		// Denominator for slope (variance term)
		let denom = data_points_count * sum_xx - sum_x * sum_x;
		let gain = (data_points_count * sum_xy - sum_x * sum_y) / denom;
		let offset = (sum_y - gain * sum_x) / data_points_count;

		LinearTransformation { adc, channel, gain, offset }
	}

	async fn save_transformation(
		&mut self,
		transformation: LinearTransformation,
	) -> Result<(), TemperatureServiceError> {
		let mut sd_card_service = self.sd_card_service.lock().await;
		let path = FileName::from_str(LINEAR_TRANSFORMATIONS_FILE_NAME).unwrap();
		if !(sd_card_service.file_exists(OperationScope::Root, path.clone())?) {
			sd_card_service.write(OperationScope::Root, path.clone(), LinearTransformation::get_csv_header())?;
		}

		sd_card_service.write(OperationScope::Root, path.clone(), transformation.to_csv_line())?;

		Ok(())
	}

	async fn prompt<T>(
		&mut self,
		prompt: &str,
	) -> Result<T, UsartError>
	where
		T: FromStr,
		<T as FromStr>::Err: core::fmt::Debug, {
		let mut serial_service = self.serial_service.lock().await;
		let mut input = String::<256>::new();
		serial_service.write_str(prompt).await?;
		serial_service.read_line(&mut input).await?;
		Ok(input.trim().parse().unwrap()) // In a real implementation, handle parse errors gracefully
	}

	async fn send_message(
		&mut self,
		message: &str,
	) -> Result<(), UsartError> {
		let mut serial_service = self.serial_service.lock().await;
		serial_service.write_str(message).await
	}
}

// Represents a single calibration data point used to perform a linear regression
#[derive(Debug, Clone, Copy, Format)]
pub struct CalibrationDataPoint {
	// Expected temperature value in degrees Celsius measured by the calibration instrument
	pub expected_temperature: f64,

	// Value measured by the ADC
	pub measured_temperature: f64,
}
