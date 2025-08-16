use ads126x::{
    register::{DataRate, Mode1Register, Mode2Register},
    ADCCommand, Ads126x,
};

use defmt::info;
use embassy_stm32::gpio::Output;
use embassy_stm32::spi::Spi;
use embedded_hal_1::delay::DelayNs;
use embedded_hal_1::digital::OutputPin;
use heapless::Vec;

#[derive(Clone, Copy)]
struct AdcSensor {
    negative: ads126x::register::NegativeInpMux,
    positive: ads126x::register::PositiveInpMux,
}

// There is an option to use interrupts using the data ready pins, but for now we will poll.
pub struct AdcManager<GpioPin>
where
    GpioPin: OutputPin,
{
    pub spi: Spi<'static, embassy_stm32::mode::Blocking>,
    pub adc1: Ads126x<GpioPin>,
    pub adc2: Ads126x<GpioPin>,
    pub adc1_cs: GpioPin,
    pub adc2_cs: GpioPin,
    #[cfg(any(feature = "strain", feature = "pressure"))]
    pub adc1_sensors: (u8, Vec<AdcSensor, 1>),
    #[cfg(feature = "temperature")]
    pub adc1_sensors: (u8, Vec<AdcSensor, 1>),
    #[cfg(any(feature = "strain", feature = "pressure"))]
    pub adc2_sensors: (u8, Vec<AdcSensor, 5>),
    #[cfg(feature = "temperature")]
    pub adc2_sensors: (u8, Vec<AdcSensor, 4>),
}

impl<GpioPin> AdcManager<GpioPin>
where
    GpioPin: OutputPin,
{
    pub fn new(
        spi: Spi<'static, embassy_stm32::mode::Blocking>,
        adc1_rst: GpioPin,
        adc2_rst: GpioPin,
        adc1_cs: GpioPin,
        adc2_cs: GpioPin,
    ) -> Self {
        Self {
            spi,
            adc1: Ads126x::new(adc1_rst),
            adc2: Ads126x::new(adc2_rst),
            adc1_cs,
            adc2_cs,
            #[cfg(feature = "strain")]
            adc1_sensors: (
                0,
                Vec::from_slice(&[
                    AdcSensor {
                        negative: ads126x::register::NegativeInpMux::AIN1,
                        positive: ads126x::register::PositiveInpMux::AIN0,
                    },
                    // AdcSensor {
                    //     negative: ads126x::register::NegativeInpMux::AIN2,
                    //     positive: ads126x::register::PositiveInpMux::AIN3,
                    // },
                    // AdcSensor {
                    //     negative: ads126x::register::NegativeInpMux::AIN4,
                    //     positive: ads126x::register::PositiveInpMux::AIN5,
                    // },
                    // AdcSensor {
                    //     negative: ads126x::register::NegativeInpMux::AIN6,
                    //     positive: ads126x::register::PositiveInpMux::AIN7,
                    // },
                    // AdcSensor {
                    //     negative: ads126x::register::NegativeInpMux::AIN8,
                    //     positive: ads126x::register::PositiveInpMux::AIN9,
                    // }
                ])
                .unwrap(),
            ),
            #[cfg(feature = "pressure")]
            adc1_sensors: (
                0,
                Vec::from_slice(&[
                    AdcSensor {
                        negative: ads126x::register::NegativeInpMux::AIN0,
                        positive: ads126x::register::PositiveInpMux::AIN1,
                    },
                    // AdcSensor {
                    //     negative: ads126x::register::NegativeInpMux::AIN2,
                    //     positive: ads126x::register::PositiveInpMux::AIN3,
                    // },
                    // AdcSensor {
                    //     negative: ads126x::register::NegativeInpMux::AIN4,
                    //     positive: ads126x::register::PositiveInpMux::AIN5,
                    // },
                    // AdcSensor {
                    //     negative: ads126x::register::NegativeInpMux::AIN6,
                    //     positive: ads126x::register::PositiveInpMux::AIN7,
                    // }
                ])
                .unwrap(),
            ),
            #[cfg(feature = "temperature")]
            adc1_sensors: (
                0,
                Vec::from_slice(&[
                    AdcSensor {
                        negative: ads126x::register::NegativeInpMux::AIN0,
                        positive: ads126x::register::PositiveInpMux::AIN1,
                    },
                    // AdcSensor {
                    //     negative: ads126x::register::NegativeInpMux::AIN2,
                    //     positive: ads126x::register::PositiveInpMux::AIN3,
                    // },
                    // AdcSensor {
                    //     negative: ads126x::register::NegativeInpMux::AIN4,
                    //     positive: ads126x::register::PositiveInpMux::AIN5,
                    // },
                    // AdcSensor {
                    //     negative: ads126x::register::NegativeInpMux::AIN6,
                    //     positive: ads126x::register::PositiveInpMux::AIN7,
                    // },
                ])
                .unwrap(),
            ),

            #[cfg(feature = "strain")]
            adc2_sensors: (
                0,
                Vec::from_slice(&[
                    AdcSensor {
                        negative: ads126x::register::NegativeInpMux::AIN0,
                        positive: ads126x::register::PositiveInpMux::AIN1,
                    },
                    AdcSensor {
                        negative: ads126x::register::NegativeInpMux::AIN2,
                        positive: ads126x::register::PositiveInpMux::AIN3,
                    },
                    AdcSensor {
                        negative: ads126x::register::NegativeInpMux::AIN4,
                        positive: ads126x::register::PositiveInpMux::AIN5,
                    },
                    AdcSensor {
                        negative: ads126x::register::NegativeInpMux::AIN6,
                        positive: ads126x::register::PositiveInpMux::AIN7,
                    },
                    AdcSensor {
                        negative: ads126x::register::NegativeInpMux::AIN8,
                        positive: ads126x::register::PositiveInpMux::AIN9,
                    },
                ])
                .unwrap(),
            ),
            #[cfg(feature = "pressure")]
            adc2_sensors: (
                0,
                Vec::from_slice(&[
                    AdcSensor {
                        negative: ads126x::register::NegativeInpMux::AIN0,
                        positive: ads126x::register::PositiveInpMux::AIN1,
                    },
                    AdcSensor {
                        negative: ads126x::register::NegativeInpMux::AIN2,
                        positive: ads126x::register::PositiveInpMux::AIN3,
                    },
                    AdcSensor {
                        negative: ads126x::register::NegativeInpMux::AIN4,
                        positive: ads126x::register::PositiveInpMux::AIN5,
                    },
                    AdcSensor {
                        negative: ads126x::register::NegativeInpMux::AIN6,
                        positive: ads126x::register::PositiveInpMux::AIN7,
                    },
                ])
                .unwrap(),
            ),
            #[cfg(feature = "temperature")]
            adc2_sensors: (
                0,
                Vec::from_slice(&[
                    AdcSensor {
                        negative: ads126x::register::NegativeInpMux::AIN0,
                        positive: ads126x::register::PositiveInpMux::AIN1,
                    },
                    AdcSensor {
                        negative: ads126x::register::NegativeInpMux::AIN2,
                        positive: ads126x::register::PositiveInpMux::AIN3,
                    },
                    AdcSensor {
                        negative: ads126x::register::NegativeInpMux::AIN4,
                        positive: ads126x::register::PositiveInpMux::AIN5,
                    },
                    AdcSensor {
                        negative: ads126x::register::NegativeInpMux::AIN6,
                        positive: ads126x::register::PositiveInpMux::AIN7,
                    },
                ])
                .unwrap(),
            ),
        }
    }

    pub fn init_adc1(
        &mut self,
        is_calibrated: bool,
        mut delay: embassy_time::Delay,
    ) -> Result<(), ads126x::error::ADS126xError> {
        self.select_adc1();
        self.adc1.set_reset_high()?;

        // 2^16 cycles of delay
        cortex_m::asm::delay(65536);

        // stop conversions
        self.adc1.send_command(ADCCommand::STOP1, &mut self.spi)?;
        self.adc1.send_command(ADCCommand::STOP2, &mut self.spi)?;

        #[cfg(any(feature = "temperature", feature = "pressure"))]
        {
            // We need to enable vbias
            let mut power_cfg = ads126x::register::PowerRegister::default();
            power_cfg.set_vbias(false);
            self.adc1.set_power(&power_cfg, &mut self.spi).unwrap();
            // Set gain
            let mut mode2_cfg = Mode2Register::default();
            mode2_cfg.set_gain(ads126x::register::PGAGain::VV1);

            self.adc1.set_mode2(&mode2_cfg, &mut self.spi).unwrap();

            let mut mode1_cfg = Mode1Register::default();
            mode1_cfg.set_sbmag(ads126x::register::SensorBiasMagnitude::R10MOhm);

            self.adc1.set_mode1(&mode1_cfg, &mut self.spi).unwrap();

            let mode1_cfg_real = self.adc1.get_mode1(&mut self.spi)?;
            let mode2_cfg_real = self.adc1.get_mode2(&mut self.spi)?;

            // verify
            info!("Mode1: {:#010b}", mode1_cfg.bits());
            info!("Mode2: {:#010b}", mode2_cfg.bits());
            info!("Mode1: {:#010b}", mode1_cfg_real.bits());
            info!("Mode2: {:#010b}", mode2_cfg_real.bits());
            assert!(mode1_cfg.difference(mode1_cfg_real).is_empty());
            assert!(mode2_cfg.difference(mode2_cfg_real).is_empty());
        }

        #[cfg(feature = "strain")]
        {
            // setup the Power register
            let mut power_cfg = ads126x::register::PowerRegister::default();
            power_cfg.clear_reset();
            self.adc1.set_power(&power_cfg, &mut self.spi)?;

            // Verify none custom config works first
            // setup mode 1 and mode 2 registers
            let mut mode1_cfg = Mode1Register::default();
            mode1_cfg.set_filter(ads126x::register::DigitalFilter::Sinc1);
            self.adc1.set_mode1(&mode1_cfg, &mut self.spi)?;

            let mut mode2_cfg = Mode2Register::default();
            mode2_cfg.set_dr(DataRate::SPS10);
            mode2_cfg.set_gain(ads126x::register::PGAGain::VV32);

            self.adc1.set_mode2(&mode2_cfg, &mut self.spi)?;

            // read back the mode1 and mode2 registers to verify
            let mode1_cfg_real = self.adc1.get_mode1(&mut self.spi)?;
            let mode2_cfg_real = self.adc1.get_mode2(&mut self.spi)?;

            // verify
            info!("Mode1: {:#010b}", mode1_cfg.bits());
            info!("Mode2: {:#010b}", mode2_cfg.bits());
            info!("Mode1: {:#010b}", mode1_cfg_real.bits());
            info!("Mode2: {:#010b}", mode2_cfg_real.bits());
            assert!(mode1_cfg.difference(mode1_cfg_real).is_empty());
            assert!(mode2_cfg.difference(mode2_cfg_real).is_empty());
        }

        if !is_calibrated {
            self.set_adc1_inpmux(&mut AdcSensor {
                negative: ads126x::register::NegativeInpMux::Float,
                positive: ads126x::register::PositiveInpMux::Float,
            });
            self.adc1.send_command(ADCCommand::SFOCAL1, &mut self.spi)?;
        }

        delay.delay_ms(1000);
        // start conversions
        self.adc1.send_command(ADCCommand::START1, &mut self.spi)?;
        self.adc1.send_command(ADCCommand::START2, &mut self.spi)?;

        self.read_adc1_data();

        Ok(())
    }

    pub fn init_adc2(
        &mut self,
        is_calibrated: bool,
        mut delay: embassy_time::Delay,
    ) -> Result<(), ads126x::error::ADS126xError> {
        self.select_adc2();
        self.adc2.set_reset_high()?;

        // 2^16 cycles of delay
        cortex_m::asm::delay(65536);

        // stop conversions
        self.adc2.send_command(ADCCommand::STOP1, &mut self.spi)?;
        self.adc2.send_command(ADCCommand::STOP2, &mut self.spi)?;

        #[cfg(any(feature = "temperature", feature = "pressure"))]
        {
            // We need to enable vbias
            let mut power_cfg = ads126x::register::PowerRegister::default();
            power_cfg.set_vbias(true);
            self.adc2.set_power(&power_cfg, &mut self.spi).unwrap();
            // Set gain
            let mut mode2_cfg = Mode2Register::default();
            mode2_cfg.set_gain(ads126x::register::PGAGain::VV32);

            self.adc2.set_mode2(&mode2_cfg, &mut self.spi).unwrap();

            let mut mode1_cfg = Mode1Register::default();
            mode1_cfg.set_sbmag(ads126x::register::SensorBiasMagnitude::R10MOhm);

            self.adc2.set_mode1(&mode1_cfg, &mut self.spi).unwrap();

            let mode1_cfg_real = self.adc2.get_mode1(&mut self.spi)?;
            let mode2_cfg_real = self.adc2.get_mode2(&mut self.spi)?;

            // verify
            info!("Mode1: {:#010b}", mode1_cfg.bits());
            info!("Mode2: {:#010b}", mode2_cfg.bits());
            info!("Mode1: {:#010b}", mode1_cfg_real.bits());
            info!("Mode2: {:#010b}", mode2_cfg_real.bits());
            assert!(mode1_cfg.difference(mode1_cfg_real).is_empty());
            assert!(mode2_cfg.difference(mode2_cfg_real).is_empty());
        }
        #[cfg(feature = "strain")]
        {
            // setup the Power register
            let mut power_cfg = ads126x::register::PowerRegister::default();
            power_cfg.clear_reset();
            self.adc2.set_power(&power_cfg, &mut self.spi)?;

            // Verify none custom config works first
            // setup mode 1 and mode 2 registers
            let mut mode1_cfg = Mode1Register::default();
            mode1_cfg.set_filter(ads126x::register::DigitalFilter::Sinc4);
            self.adc2.set_mode1(&mode1_cfg, &mut self.spi)?;

            let mut mode2_cfg = Mode2Register::default();
            mode2_cfg.set_dr(DataRate::SPS20);
            mode2_cfg.set_gain(ads126x::register::PGAGain::VV32);

            self.adc2.set_mode2(&mode2_cfg, &mut self.spi)?;

            // read back the mode1 and mode2 registers to verify
            let mode1_cfg_real = self.adc2.get_mode1(&mut self.spi)?;
            let mode2_cfg_real = self.adc2.get_mode2(&mut self.spi)?;

            // verify
            info!("Mode1: {:#010b}", mode1_cfg.bits());
            info!("Mode2: {:#010b}", mode2_cfg.bits());
            info!("Mode1: {:#010b}", mode1_cfg_real.bits());
            info!("Mode2: {:#010b}", mode2_cfg_real.bits());
            assert!(mode1_cfg.difference(mode1_cfg_real).is_empty());
            assert!(mode2_cfg.difference(mode2_cfg_real).is_empty());
        }

        if !is_calibrated {
            self.set_adc2_inpmux(&mut AdcSensor {
                negative: ads126x::register::NegativeInpMux::Float,
                positive: ads126x::register::PositiveInpMux::Float,
            });
            self.adc2.send_command(ADCCommand::SFOCAL1, &mut self.spi)?;
        }

        delay.delay_ms(1000);

        // start conversions
        self.adc2.send_command(ADCCommand::START1, &mut self.spi)?;
        self.adc2.send_command(ADCCommand::START2, &mut self.spi)?;

        self.read_adc2_data();

        Ok(())
    }

    pub fn select_adc1(&mut self) {
        self.adc2_cs.set_high();
        self.adc1_cs.set_low();
    }

    pub fn select_adc2(&mut self) {
        self.adc1_cs.set_high();
        self.adc2_cs.set_low();
    }

    pub fn set_adc1_inpmux(
        &mut self,
        sensor: &mut AdcSensor,
    ) -> Result<(), ads126x::error::ADS126xError> {
        self.select_adc1();
        let mut reg = ads126x::register::InpMuxRegister::default();
        reg.set_muxn(sensor.negative);
        reg.set_muxp(sensor.positive);
        self.adc1.set_inpmux(&reg, &mut self.spi)
    }

    pub fn set_adc2_inpmux(
        &mut self,
        sensor: &mut AdcSensor,
    ) -> Result<(), ads126x::error::ADS126xError> {
        self.select_adc2();
        let mut reg = ads126x::register::InpMuxRegister::default();
        reg.set_muxn(sensor.negative);
        reg.set_muxp(sensor.positive);
        self.adc2.set_inpmux(&reg, &mut self.spi)
    }

    /*
    There are possibly 4,5, or 6 bytes of data to read from ADC1. There is an optonal status byte first and an optional CRC/CHK byte last.
    There are possibly 3,4, or 5 bytes of data to read from ADC2. There is an optonal status byte first and a fixed-value byte equal to 00h (zero pad byte) and an optional CRC/CHK byte.
    We can poll and just keep checking the ADC1 or ADC2 new data bit.
    ADC does not respond to commands until the read operation is complete, or terminated by CS going high.
    The data bytes are from the 32-bit conversion word.

     */

    pub fn read_adc1_data(&mut self) -> Result<(u8, i32), ads126x::error::ADS126xError> {
        self.select_adc1();
        // ask for data
        /*
        If no command is intended, keep the DIN pin low during readback. If an input command is
        sent during readback, the ADC executes the command, and data interruption may result.
         */
        let data = self.adc1.read_data1(&mut self.spi)?;
        // select the next sensor based on round robin
        let current_index = (self.adc1_sensors.0 as usize + 1) % self.adc1_sensors.1.len();

        // // set the inputmux
        // let mut sensor = &mut self.adc1_sensors.1[current_index].clone();
        // self.set_adc1_inpmux(&mut sensor).unwrap();

        // // update the index
        // self.adc1_sensors.0 = current_index as u8;

        Ok((current_index as u8, data))
    }

    pub fn read_adc2_data(&mut self) -> Result<(u8, i32), ads126x::error::ADS126xError> {
        self.select_adc2();
        // ask for data
        /*
        If no command is intended, keep the DIN pin low during readback. If an input command is
        sent during readback, the ADC executes the command, and data interruption may result.
         */
        let data = self.adc2.read_data1(&mut self.spi)?;
        // select the next sensor based on round robin
        let current_index = (self.adc2_sensors.0 as usize + 1) % self.adc2_sensors.1.len();

        // set the inputmux
        let mut sensor = &mut self.adc2_sensors.1[current_index].clone();
        self.set_adc2_inpmux(&mut sensor).unwrap();

        // update the index
        self.adc2_sensors.0 = current_index as u8;

        Ok((current_index as u8, data))
    }
}
