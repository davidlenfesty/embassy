#![no_std]
#![no_main]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

use core::mem;
use core::task::Waker;
use defmt::*;
use embassy::blocking_mutex::raw::ThreadModeRawMutex;
use embassy::channel::Channel;
use embassy::executor::Spawner;
use embassy::util::Forever;
use embassy_net::{PacketBox, PacketBoxExt, PacketBuf};
use embassy_nrf::pac;
use embassy_nrf::usb::Driver;
use embassy_nrf::Peripherals;
use embassy_nrf::{interrupt, peripherals};
use embassy_usb::{Builder, Config, UsbDevice};
use embassy_usb_ncm::{CdcNcmClass, Receiver, Sender, State};

use defmt_rtt as _; // global logger
use panic_probe as _;

type MyDriver = Driver<'static, peripherals::USBD>;

#[embassy::task]
async fn usb_task(mut device: UsbDevice<'static, MyDriver>) -> ! {
    device.run().await
}

#[embassy::task]
async fn usb_ncm_rx_task(mut class: Receiver<'static, MyDriver>) {
    loop {
        class.wait_connection().await;
        info!("Connected");
        loop {
            let mut p = unwrap!(PacketBox::new(embassy_net::Packet::new()));
            let n = match class.read_packet(&mut p[..]).await {
                Ok(n) => n,
                Err(e) => {
                    warn!("error reading packet: {:?}", e);
                    break;
                }
            };
            let buf = p.slice(0..n);
            if RX_CHANNEL.try_send(buf).is_err() {
                warn!("Failed pushing rx'd packet to channel.");
            }
        }
    }
}

#[embassy::task]
async fn usb_ncm_tx_task(mut class: Sender<'static, MyDriver>) {
    loop {
        let pkt = TX_CHANNEL.recv().await;
        if let Err(e) = class.write_packet(&pkt[..]).await {
            warn!("Failed to TX packet: {:?}", e);
        }
    }
}

#[embassy::task]
async fn net_task() -> ! {
    embassy_net::run().await
}

#[embassy::main]
async fn main(spawner: Spawner, p: Peripherals) {
    let clock: pac::CLOCK = unsafe { mem::transmute(()) };
    let power: pac::POWER = unsafe { mem::transmute(()) };

    info!("Enabling ext hfosc...");
    clock.tasks_hfclkstart.write(|w| unsafe { w.bits(1) });
    while clock.events_hfclkstarted.read().bits() != 1 {}

    info!("Waiting for vbus...");
    while !power.usbregstatus.read().vbusdetect().is_vbus_present() {}
    info!("vbus OK");

    // Create the driver, from the HAL.
    let irq = interrupt::take!(USBD);
    let driver = Driver::new(p.USBD, irq);

    // Create embassy-usb Config
    let config = Config::new(0xc0de, 0xcafe);

    struct Resources {
        device_descriptor: [u8; 256],
        config_descriptor: [u8; 256],
        bos_descriptor: [u8; 256],
        control_buf: [u8; 128],
        serial_state: State<'static>,
    }
    static RESOURCES: Forever<Resources> = Forever::new();
    let res = RESOURCES.put(Resources {
        device_descriptor: [0; 256],
        config_descriptor: [0; 256],
        bos_descriptor: [0; 256],
        control_buf: [0; 128],
        serial_state: State::new(),
    });

    // Create embassy-usb DeviceBuilder using the driver and config.
    let mut builder = Builder::new(
        driver,
        config,
        &mut res.device_descriptor,
        &mut res.config_descriptor,
        &mut res.bos_descriptor,
        &mut res.control_buf,
        None,
    );

    // Create classes on the builder.
    let class = CdcNcmClass::new(&mut builder, &mut res.serial_state, 64);

    // Build the builder.
    let usb = builder.build();

    unwrap!(spawner.spawn(usb_task(usb)));

    let (tx, rx) = class.split();
    unwrap!(spawner.spawn(usb_ncm_rx_task(rx)));
    unwrap!(spawner.spawn(usb_ncm_tx_task(tx)));

    // Init embassy-net
    struct NetResources {
        resources: embassy_net::StackResources<1, 2, 8>,
        configurator: embassy_net::DhcpConfigurator,
        device: Device,
    }
    static NET_RESOURCES: Forever<NetResources> = Forever::new();
    let res = NET_RESOURCES.put(NetResources {
        resources: embassy_net::StackResources::new(),
        configurator: embassy_net::DhcpConfigurator::new(),
        device: Device {
            mac_addr: [2, 2, 2, 2, 2, 2],
        },
    });
    embassy_net::init(&mut res.device, &mut res.configurator, &mut res.resources);
    unwrap!(spawner.spawn(net_task()));

    // And now we can use it! yay
    // TODO
}

static TX_CHANNEL: Channel<ThreadModeRawMutex, PacketBuf, 8> = Channel::new();
static RX_CHANNEL: Channel<ThreadModeRawMutex, PacketBuf, 8> = Channel::new();

struct Device {
    mac_addr: [u8; 6],
}

impl embassy_net::Device for Device {
    fn register_waker(&mut self, waker: &Waker) {
        // loopy loopy wakey wakey
        waker.wake_by_ref()
    }

    fn link_state(&mut self) -> embassy_net::LinkState {
        embassy_net::LinkState::Up
    }

    fn capabilities(&mut self) -> embassy_net::DeviceCapabilities {
        let mut caps = embassy_net::DeviceCapabilities::default();
        caps.max_transmission_unit = 1514; // 1500 IP + 14 ethernet header
        caps.medium = embassy_net::Medium::Ethernet;
        caps
    }

    fn is_transmit_ready(&mut self) -> bool {
        true
    }

    fn transmit(&mut self, pkt: PacketBuf) {
        if TX_CHANNEL.try_send(pkt).is_err() {
            warn!("TX failed")
        }
    }

    fn receive<'a>(&mut self) -> Option<PacketBuf> {
        RX_CHANNEL.try_recv().ok()
    }

    fn ethernet_address(&mut self) -> [u8; 6] {
        self.mac_addr
    }
}

#[no_mangle]
fn _embassy_rand(buf: &mut [u8]) {
    // TODO
    buf.fill(0x42)
}
