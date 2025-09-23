use core::mem::MaybeUninit;

use embassy_stm32::rcc::{AHBPrescaler, APBPrescaler, HSIPrescaler, Pll, PllDiv, PllMul, PllPreDiv, PllSource, Sysclk, VoltageScale};
/// Any HAL configuration and setup goes here.
use embassy_stm32::{init, Config, Peripherals};
pub use embedded_alloc::LlffHeap as Heap;

const HEAP_SIZE: usize = 40000;

#[global_allocator]
pub static HEAP: Heap = Heap::empty();

pub fn configure_hal() -> Peripherals {
	static mut HEAP_MEMORY: [MaybeUninit<u8>; HEAP_SIZE] = [MaybeUninit::uninit(); HEAP_SIZE];

	#[allow(static_mut_refs)]
	unsafe {
		HEAP.init(HEAP_MEMORY.as_ptr() as usize, HEAP_SIZE)
	}

	let mut config = Config::default();

	config.rcc.hsi = Some(HSIPrescaler::DIV1);
	config.rcc.csi = true;

	config.rcc.pll1 = Some(Pll {
		source: PllSource::HSI,   // 64 MHz
		prediv: PllPreDiv::DIV4,  // 16 MHz
		mul: PllMul::MUL50,       // 800 MHz
		divp: Some(PllDiv::DIV2), // 400 Mhz
		divq: Some(PllDiv::DIV8), // 100 MHz
		divr: None,
	});

	config.rcc.pll2 = Some(Pll {
		source: PllSource::HSI,   // 64 MHz
		prediv: PllPreDiv::DIV4,  // 16 MHz
		mul: PllMul::MUL50,       // 800 MHz
		divp: Some(PllDiv::DIV2), // 400 Mhz
		divq: Some(PllDiv::DIV8), // 100 MHz
		divr: None,
	});

	config.rcc.sys = Sysclk::PLL1_P; // 400 Mhz
	config.rcc.ahb_pre = AHBPrescaler::DIV2; // 200 Mhz
	config.rcc.apb1_pre = APBPrescaler::DIV2; // 100 Mhz
	config.rcc.apb2_pre = APBPrescaler::DIV2; // 100 Mhz
	config.rcc.apb3_pre = APBPrescaler::DIV2; // 100 Mhz
	config.rcc.apb4_pre = APBPrescaler::DIV2; // 100 Mhz
	config.rcc.voltage_scale = VoltageScale::Scale1;

	init(config)
}
