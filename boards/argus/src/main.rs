#![feature(impl_trait_in_assoc_type)]
#![no_std]
#![no_main]

// #[cfg(not(any(feature = "pressure", feature = "temperature", feature = "strain")))]
// compile_error!(
// 	"You must enable exactly one of the features: 'pressure', 'temperature', or 'strain'."
// );

use argus::adc::service::{AdcConfig, AdcService};
use argus::sd::service::SDCardService;
use argus::sd::task::sd_card_task;
use argus::serial::service::SerialService;
use argus::state_machine::service::{StateMachineOrchestrator, StateMachineWorker};
use argus::state_machine::types::Events;
use argus::utils::hal::configure_hal;
use argus::utils::types::AsyncMutex;
use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::gpio::Pin;
use embassy_stm32::{bind_interrupts, peripherals, usart};
use embassy_time::Timer;
use panic_probe as _;
use static_cell::StaticCell;

// Mapping of NVIC interrupts to Embassy interrupt handlers
bind_interrupts!(struct InterruptRequests {
	UART7 => usart::InterruptHandler<peripherals::UART7>;
});

// All services are singletons held in a static cell to initialize after peripherals are available
// And wrapped around a mutex so they can be accessed safely from multiple async tasks
static SD_CARD_SERVICE: StaticCell<AsyncMutex<SDCardService>> = StaticCell::new();
static ADC_SERVICE: StaticCell<AsyncMutex<AdcService>> = StaticCell::new();
static SERIAL_SERVICE: StaticCell<AsyncMutex<SerialService>> = StaticCell::new();

static STATE_MACHINE_ORCHESTRATOR: StaticCell<AsyncMutex<StateMachineOrchestrator>> = StaticCell::new();

#[cfg(feature = "temperature")]
static TEMPERATURE_SERVICE: StaticCell<AsyncMutex<argus::temperature::service::TemperatureService>> = StaticCell::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
	info!("Starting up...");
	let peripherals = configure_hal();
	let sd_card_service = SD_CARD_SERVICE.init(AsyncMutex::new(SDCardService::new(
		peripherals.SPI1,
		peripherals.PA5,
		peripherals.PA7,
		peripherals.PA6,
		peripherals.PC4,
	)));
	let adc_service = ADC_SERVICE.init(AsyncMutex::new(AdcService::new(
		peripherals.SPI4,
		peripherals.PE2,
		peripherals.PE6,
		peripherals.PE5,
		peripherals.DMA1_CH0,
		peripherals.DMA1_CH1,
		[
			AdcConfig {
				chip_select: peripherals.PE1.degrade(),
				data_ready: peripherals.PB9.degrade(),
				reset: peripherals.PE0.degrade(),
				start: peripherals.PB0.degrade(),
			},
			AdcConfig {
				chip_select: peripherals.PB8.degrade(),
				data_ready: peripherals.PB6.degrade(),
				reset: peripherals.PB7.degrade(),
				start: peripherals.PB1.degrade(),
			},
		],
	)));
	let serial_service = SERIAL_SERVICE.init(AsyncMutex::new(
		SerialService::new(
			peripherals.UART7,
			peripherals.PE8,
			peripherals.PE7,
			InterruptRequests,
			peripherals.DMA1_CH2,
			peripherals.DMA1_CH3,
			115200,
		)
		.unwrap(),
	));

	let state_machine_orchestrator = STATE_MACHINE_ORCHESTRATOR.init(AsyncMutex::new(StateMachineOrchestrator::new()));

	// General tasks that must run regardless of board type
	spawner.must_spawn(sd_card_task(sd_card_service));

	// Spawn tasks needed for temperature board
	#[cfg(feature = "temperature")]
	{
		// Imported inside the block to avoid unused leaking the import when the feature is not enabled
		use argus::temperature::service::TemperatureService;
		use argus::temperature::tasks;

		let temperature_service = TEMPERATURE_SERVICE.init(AsyncMutex::new(TemperatureService::new(adc_service, sd_card_service, serial_service)));

		// Setup the temperature service before starting the tasks
		temperature_service.lock().await.setup().await.unwrap();

		spawner.must_spawn(tasks::measure_rtds(
			StateMachineWorker::new(state_machine_orchestrator),
			temperature_service,
		));
		spawner.must_spawn(tasks::measure_thermocouples(
			StateMachineWorker::new(state_machine_orchestrator),
			temperature_service,
		));
		spawner.must_spawn(tasks::log_measurements(
			StateMachineWorker::new(state_machine_orchestrator),
			sd_card_service,
		));
	}

	// Immediately request to start recording
	state_machine_orchestrator.lock().await.dispatch_event(Events::StartRecordingRequested);

	Timer::after_secs(30).await;

	state_machine_orchestrator.lock().await.dispatch_event(Events::StopRecordingRequested);
}
