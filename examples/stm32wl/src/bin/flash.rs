#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use core::ptr::read_volatile;
use defmt::{info, unwrap};
use embassy::executor::Spawner;
use embassy::time::{Duration, Timer};
use embassy_stm32::flash::Flash;
use embassy_stm32::Peripherals;
use embedded_storage::nor_flash::{NorFlash, ReadNorFlash};

use defmt_rtt as _; // global logger
use panic_probe as _;

fn config() -> embassy_stm32::Config {
    let mut config = embassy_stm32::Config::default();
    config.rcc.enable_flash = true;
    config
}

#[embassy::main(config = "config()")]
async fn main(_spawner: Spawner, p: Peripherals) {
    info!("Hello Flash!");

    // probe-run breaks without this, I'm not sure why.
    Timer::after(Duration::from_secs(1)).await;

    const ADDR: u32 = 0x8036000;

    let mut f = Flash::new(p.FLASH);

    info!("Reading...");
    let mut buf = [0u8; 8];
    unwrap!(f.read(ADDR, &mut buf));
    info!("Read: {=[u8]:x}", buf);

    info!("Erasing...");
    unwrap!(f.erase(ADDR, ADDR + 2048));

    info!("Reading...");
    let mut buf = [0u8; 8];
    unwrap!(f.read(ADDR, &mut buf));
    info!("Read: {=[u8]:x}", buf);

    info!("Writing...");
    unwrap!(f.write(ADDR, &[1, 2, 3, 4, 5, 6, 7, 8]));

    info!("Reading...");
    let mut buf = [0u8; 8];
    unwrap!(f.read(ADDR, &mut buf));
    info!("Read: {=[u8]:x}", buf);
    assert_eq!(&buf[..], &[1, 2, 3, 4, 5, 6, 7, 8]);
}
