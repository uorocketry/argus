#![no_std]
#![no_main]

use defmt::info;
use panic_probe as _;
use stm32h7xx_hal::{pac, rcc};
use stm32h7xx_hal::prelude::*;

#[defmt_test::tests]
mod tests {
    use super::*;

    #[init]
    fn init() {
        let _cp = cortex_m::Peripherals::take().unwrap();
        let dp = pac::Peripherals::take().unwrap();

        let pwr = dp.PWR.constrain();
        let pwrcfg = pwr.freeze();

        info!("Power enabled");
        // RCC
        let mut rcc = dp.RCC.constrain();
        let reset = rcc.get_reset_reason();

        let ccdr = rcc
            // .use_hse(48.MHz()) // check the clock hardware
            .sys_ck(200.MHz())
            .pll1_strategy(rcc::PllConfigStrategy::Iterative)
            .pll1_q_ck(32.MHz())
            .freeze(pwrcfg, &dp.SYSCFG);

        info!("Reset reason: {:?}", reset);
    }

    #[test]
    fn simple_test() {
        assert!(true == true)
    }
}
