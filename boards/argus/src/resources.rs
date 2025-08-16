// =================================================================================
// Shared Resources & Types
// =================================================================================

use crate::state_machine::Events;
use core::cell::RefCell;
use embassy_stm32::rtc::Rtc;
use embassy_stm32::spi::Spi;
use embassy_stm32::{bind_interrupts, mode, peripherals, usart};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::channel::Channel;
use embassy_time::Instant;
use embedded_alloc::LlffHeap as Heap;
use messages_prost::sbg::SbgData;
use static_cell::StaticCell;


pub const SD_BUFFER_SIZE: usize = 255;

#[global_allocator]
pub static HEAP: Heap = Heap::empty();

pub static SD_CHANNEL: Channel<CriticalSectionRawMutex, (&str, [u8; SD_BUFFER_SIZE]), 5> = Channel::new(); // file name, data
pub static EVENT_CHANNEL: Channel<CriticalSectionRawMutex, Events, 2> = Channel::new();

// The SPI bus is protected by a Mutex, so the RefCell is not needed.
pub static SPI_BUS: StaticCell<
    embassy_sync::mutex::Mutex<CriticalSectionRawMutex, Spi<mode::Async>>,
> = StaticCell::new();
pub static SPI_BUS_CELL: StaticCell<RefCell<Spi<mode::Blocking>>> = StaticCell::new();

// Static variable for the RTC
pub static RTC: Mutex<CriticalSectionRawMutex, RefCell<Option<Rtc>>> =
    Mutex::new(RefCell::new(None));

bind_interrupts!(struct Irqs {
    UART7 => usart::InterruptHandler<peripherals::UART7>;
    UART8 => usart::InterruptHandler<peripherals::UART8>;
});