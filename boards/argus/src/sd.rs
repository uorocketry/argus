use crate::resources::{SD_CHANNEL, SPI_BUS_CELL};
use core::cell::RefCell;
use core::marker::PhantomData;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::mode::Blocking;
use embassy_stm32::peripherals::{PA5, PA6, PA7, PC4, PE9, SPI1};
use embassy_stm32::spi::{BitOrder, Spi};
use embassy_stm32::time::mhz;
use embassy_time::Delay;
use embedded_hal_bus::spi::RefCellDevice;
use embedded_sdmmc::{BlockDevice, SdCard, VolumeManager};

pub struct TimeSink {
    _marker: PhantomData<*const ()>,
}

impl TimeSink {
    fn new() -> Self {
        TimeSink {
            _marker: PhantomData,
        }
    }
}

impl embedded_sdmmc::TimeSource for TimeSink {
    fn get_timestamp(&self) -> embedded_sdmmc::Timestamp {
        embedded_sdmmc::Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

pub fn setup_sdmmc_interface(
    spi: SPI1,
    sck: PA5,
    mosi: PA7,
    miso: PA6,
    cs: PC4,
) -> SdCard<RefCellDevice<'static, Spi<'static, Blocking>, Output<'static>, Delay>, Delay> {
    let mut sd_spi_config = embassy_stm32::spi::Config::default();

    sd_spi_config.frequency = mhz(16);
    sd_spi_config.bit_order = BitOrder::MsbFirst;

    let mut sd_spi_bus = Spi::new_blocking(spi, sck, mosi, miso, sd_spi_config);

    let sd_cs = Output::new(cs, Level::High, Speed::Low);
    let data: [u8; 10] = [0xFF; 10];
    sd_spi_bus.blocking_write(&data).unwrap();
    //
    // let sd_spi_bus_ref_cell = RefCell::new(sd_spi_bus);
    // let sd_spi_device = RefCellDevice::new(&sd_spi_bus_ref_cell, sd_cs, Delay);
    let spi_bus_ref = SPI_BUS_CELL.init(RefCell::new(sd_spi_bus));
    let sd_spi_device = RefCellDevice::new(spi_bus_ref, sd_cs, Delay).unwrap();

    SdCard::new(sd_spi_device, Delay)
}

#[embassy_executor::task]
pub async fn sdmmc_task(
    sd: embedded_sdmmc::SdCard<
        embedded_hal_bus::spi::RefCellDevice<
            'static,
            Spi<'static, embassy_stm32::mode::Blocking>,
            Output<'static>,
            Delay,
        >,
        Delay,
    >,
) {
    // setup the directory object
    let volume_mgr = VolumeManager::new(sd, TimeSink::new());
    if let Ok(volume0) = volume_mgr.open_volume(embedded_sdmmc::VolumeIdx(0)) {
        // should never fail
        let root_dir = volume0.open_root_dir().unwrap();

        loop {
            let (file, data) = SD_CHANNEL.receive().await;
            if let Ok(file) =
                root_dir.open_file_in_dir(file, embedded_sdmmc::Mode::ReadWriteCreateOrAppend)
            {
                match file.write(&data) {
                    Err(_) => {
                        todo!("Log to radio we failed to write.");
                    }
                    _ => {}
                }
                file.flush();
            }
        }
    } else {
        // Log to the radio in the event this happens.
        // Could trigger the fault state.
        todo!("Write to the radio Fault");
    };
}
