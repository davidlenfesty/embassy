#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use embassy::time::{Duration, Timer};
use embassy_boot_stm32::updater;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::flash::Flash;
use embassy_stm32::gpio::{Input, Level, Output, Pull, Speed};
use embassy_stm32::Peripherals;
use embassy_traits::adapter::BlockingAsync;
use panic_reset as _;

use defmt_rtt::*;

static APP_B: &[u8] = include_bytes!("../../b.bin");

#[embassy::main]
async fn main(_s: embassy::executor::Spawner, p: Peripherals) {
    let flash = Flash::new(p.FLASH);
    let mut flash = BlockingAsync::new(flash);

    let button = Input::new(p.PA0, Pull::Up);
    let mut button = ExtiInput::new(button, p.EXTI0);

    let mut led = Output::new(p.PB9, Level::Low, Speed::Low);

    defmt::info!("Waiting for updating");
    loop {
        Timer::after(Duration::from_secs(5)).await;
        defmt::info!("Writing firmware");
        let mut updater = updater::new();
        let mut offset = 0;
        for chunk in APP_B.chunks(2048) {
            let mut buf: [u8; 2048] = [0; 2048];
            buf[..chunk.len()].copy_from_slice(chunk);
            updater
                .write_firmware(offset, &buf, &mut flash, 2048)
                .await
                .unwrap();
            offset += chunk.len();
        }
        defmt::info!("Done writing firmware, marking update");
        updater.mark_update(&mut flash).await.unwrap();
        defmt::info!("Done!");
        led.set_high();
        defmt::info!("Waiting for updating");
        loop {
            //    cortex_m::peripheral::SCB::sys_reset();
        }
    }
}
