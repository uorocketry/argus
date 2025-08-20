// =================================================================================
// Shared Resources & Types
// =================================================================================

use crate::state_machine::Events;
use core::cell::RefCell;
use embassy_stm32::rtc::Rtc;
use embassy_stm32::spi::Spi;
use embassy_stm32::{bind_interrupts, can, mode, peripherals, usart};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::channel::Channel;
use embedded_alloc::LlffHeap as Heap;
use static_cell::StaticCell;

pub const SD_BUFFER_SIZE: usize = 255;

#[global_allocator]
pub static HEAP: Heap = Heap::empty();

pub static SD_CHANNEL: Channel<CriticalSectionRawMutex, (&str, [u8; SD_BUFFER_SIZE]), 5> =
    Channel::new(); // file name, data
pub static EVENT_CHANNEL: Channel<CriticalSectionRawMutex, Events, 2> = Channel::new();

pub static SPI_BUS_CELL: StaticCell<RefCell<Spi<mode::Blocking>>> = StaticCell::new();

pub static ADC_SPI_BUS_CELL: StaticCell<RefCell<Spi<mode::Blocking>>> = StaticCell::new();

// Static variable for the RTC
pub static RTC: Mutex<CriticalSectionRawMutex, RefCell<Option<Rtc>>> =
    Mutex::new(RefCell::new(None));

bind_interrupts!(pub struct Irqs {
    UART7 => usart::InterruptHandler<peripherals::UART7>;
    UART8 => usart::InterruptHandler<peripherals::UART8>;
    FDCAN2_IT0 => can::IT0InterruptHandler<peripherals::FDCAN2>;
    FDCAN2_IT1 => can::IT1InterruptHandler<peripherals::FDCAN2>;
});
