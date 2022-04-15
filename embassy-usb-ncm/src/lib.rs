#![no_std]
#![feature(generic_associated_types)]
#![feature(type_alias_impl_trait)]

// This mod MUST go first, so that the others see its macros.
pub(crate) mod fmt;

use core::intrinsics::copy_nonoverlapping;
use core::mem::{size_of, MaybeUninit};
use core::sync::atomic::{AtomicBool, Ordering};
use embassy::time::{Duration, Timer};
use embassy::waitqueue::AtomicWaker;
use embassy_usb::control::{self, ControlHandler, InResponse, OutResponse, Request};
use embassy_usb::driver::{Endpoint, EndpointError, EndpointIn, EndpointOut};
use embassy_usb::{driver::Driver, types::*, Builder};

/// This should be used as `device_class` when building the `UsbDevice`.
pub const USB_CLASS_CDC: u8 = 0x02;

const USB_CLASS_CDC_DATA: u8 = 0x0a;
const CDC_SUBCLASS_NCM: u8 = 0x0d;

const CDC_PROTOCOL_NONE: u8 = 0x00;
const CDC_PROTOCOL_NTB: u8 = 0x01;

const CS_INTERFACE: u8 = 0x24;
const CDC_TYPE_HEADER: u8 = 0x00;
const CDC_TYPE_UNION: u8 = 0x06;
const CDC_TYPE_ETHERNET: u8 = 0x0F;
const CDC_TYPE_NCM: u8 = 0x1A;

const REQ_SEND_ENCAPSULATED_COMMAND: u8 = 0x00;
const REQ_GET_ENCAPSULATED_COMMAND: u8 = 0x01;
const REQ_SET_ETHERNET_MULTICAST_FILTERS: u8 = 0x40;
const REQ_SET_ETHERNET_POWER_MANAGEMENT_PATTERN_FILTER: u8 = 0x41;
const REQ_GET_ETHERNET_POWER_MANAGEMENT_PATTERN_FILTER: u8 = 0x42;
const REQ_SET_ETHERNET_PACKET_FILTER: u8 = 0x43;
const REQ_GET_ETHERNET_STATISTIC: u8 = 0x44;
const REQ_GET_NTB_PARAMETERS: u8 = 0x80;
const REQ_GET_NET_ADDRESS: u8 = 0x81;
const REQ_SET_NET_ADDRESS: u8 = 0x82;
const REQ_GET_NTB_FORMAT: u8 = 0x83;
const REQ_SET_NTB_FORMAT: u8 = 0x84;
const REQ_GET_NTB_INPUT_SIZE: u8 = 0x85;
const REQ_SET_NTB_INPUT_SIZE: u8 = 0x86;
const REQ_GET_MAX_DATAGRAM_SIZE: u8 = 0x87;
const REQ_SET_MAX_DATAGRAM_SIZE: u8 = 0x88;
const REQ_GET_CRC_MODE: u8 = 0x89;
const REQ_SET_CRC_MODE: u8 = 0x8A;

const NOTIF_MAX_PACKET_SIZE: u16 = 8;
const NOTIF_POLL_INTERVAL: u8 = 20;

const NTB_MAX_SIZE: usize = 1600;
const NTH_SIG: u32 = 0x484d434e;
const NDP_SIG: u32 = 0x304d434e;

const ALTERNATE_SETTING_DISABLED: u8 = 0x00;
const ALTERNATE_SETTING_ENABLED: u8 = 0x01;

/// Simple NTB header (NTH+NDP all in one) for sending packets
#[repr(packed)]
struct NtbOutHeader {
    // NTH
    nth_sig: u32,
    nth_len: u16,
    nth_seq: u16,
    nth_total_len: u16,
    nth_first_index: u16,

    // NDP
    ndp_sig: u32,
    ndp_len: u16,
    ndp_next_index: u16,
    ndp_datagram_index: u16,
    ndp_datagram_len: u16,
    ndp_term1: u16,
    ndp_term2: u16,
}

#[repr(packed)]
struct NtbParameters {
    length: u16,
    formats_supported: u16,
    in_params: NtbParametersDir,
    out_params: NtbParametersDir,
}

#[repr(packed)]
struct NtbParametersDir {
    max_size: u32,
    divisor: u16,
    payload_remainder: u16,
    out_alignment: u16,
    max_datagram_count: u16,
}

fn byteify<T>(buf: &mut [u8], data: T) -> &[u8] {
    let len = size_of::<T>();
    unsafe { copy_nonoverlapping(&data as *const _ as *const u8, buf.as_mut_ptr(), len) }
    &buf[..len]
}

pub struct State<'a> {
    comm_control: MaybeUninit<CommControl<'a>>,
    data_control: MaybeUninit<DataControl<'a>>,
    shared: ControlShared,
}

impl<'a> State<'a> {
    pub fn new() -> Self {
        Self {
            comm_control: MaybeUninit::uninit(),
            data_control: MaybeUninit::uninit(),
            shared: Default::default(),
        }
    }
}

/// Shared data between Control and CdcAcmClass
struct ControlShared {
    enabled: AtomicBool,
    rx_waker: AtomicWaker,
    tx_waker: AtomicWaker,
}

impl Default for ControlShared {
    fn default() -> Self {
        ControlShared {
            enabled: AtomicBool::new(false),
            rx_waker: AtomicWaker::new(),
            tx_waker: AtomicWaker::new(),
        }
    }
}

struct CommControl<'a> {
    shared: &'a ControlShared,
}

impl<'d> ControlHandler for CommControl<'d> {
    fn reset(&mut self) {
        self.shared.enabled.store(false, Ordering::SeqCst);
        self.shared.rx_waker.wake();
        self.shared.tx_waker.wake();
    }

    fn control_out(&mut self, req: control::Request, data: &[u8]) -> OutResponse {
        match req.request {
            REQ_SEND_ENCAPSULATED_COMMAND => {
                // We don't actually support encapsulated commands but pretend we do for standards
                // compatibility.
                OutResponse::Accepted
            }
            REQ_SET_NTB_INPUT_SIZE => {
                // TODO
                OutResponse::Accepted
            }
            _ => OutResponse::Rejected,
        }
    }

    fn control_in<'a>(&'a mut self, req: Request, buf: &'a mut [u8]) -> InResponse<'a> {
        match req.request {
            REQ_GET_NTB_PARAMETERS => {
                let res = NtbParameters {
                    length: size_of::<NtbParameters>() as _,
                    formats_supported: 1, // only 16bit,
                    in_params: NtbParametersDir {
                        max_size: NTB_MAX_SIZE as _,
                        divisor: 4,
                        payload_remainder: 0,
                        out_alignment: 4,
                        max_datagram_count: 0, // not used
                    },
                    out_params: NtbParametersDir {
                        max_size: NTB_MAX_SIZE as _,
                        divisor: 4,
                        payload_remainder: 0,
                        out_alignment: 4,
                        max_datagram_count: 20, // arbitrary, any amount supported really.
                    },
                };
                InResponse::Accepted(byteify(buf, res))
            }
            _ => InResponse::Rejected,
        }
    }
}

struct DataControl<'a> {
    shared: &'a ControlShared,
}

impl<'d> ControlHandler for DataControl<'d> {
    fn set_alternate_setting(&mut self, alternate_setting: u8) {
        match alternate_setting {
            ALTERNATE_SETTING_ENABLED => {
                info!("interface alt set to ENABLED");
                self.shared.enabled.store(true, Ordering::SeqCst);
                self.shared.rx_waker.wake();
                self.shared.tx_waker.wake();
            }
            ALTERNATE_SETTING_DISABLED => {
                info!("interface alt set to DISABLED");
                self.shared.enabled.store(false, Ordering::SeqCst);
                self.shared.rx_waker.wake();
                self.shared.tx_waker.wake();
            }
            _ => unreachable!(),
        }
    }
}

pub struct CdcNcmClass<'d, D: Driver<'d>> {
    _comm_if: InterfaceNumber,
    comm_ep: D::EndpointIn,

    data_if: InterfaceNumber,
    read_ep: D::EndpointOut,
    write_ep: D::EndpointIn,

    control: &'d ControlShared,
}

impl<'d, D: Driver<'d>> CdcNcmClass<'d, D> {
    /// Creates a new CdcAcmClass with the provided UsbBus and max_packet_size in bytes. For
    /// full-speed devices, max_packet_size has to be one of 8, 16, 32 or 64.
    pub fn new(
        builder: &mut Builder<'d, D>,
        state: &'d mut State<'d>,
        max_packet_size: u16,
    ) -> Self {
        let comm_control = state.comm_control.write(CommControl {
            shared: &state.shared,
        });
        let data_control = state.data_control.write(DataControl {
            shared: &state.shared,
        });

        let control_shared = &state.shared;

        let mut func = builder.function(USB_CLASS_CDC, CDC_SUBCLASS_NCM, CDC_PROTOCOL_NONE);

        // Control interface
        let mut iface = func.interface(Some(comm_control));
        let comm_if = iface.interface_number();
        let mut alt = iface.alt_setting(USB_CLASS_CDC, CDC_SUBCLASS_NCM, CDC_PROTOCOL_NONE);

        alt.descriptor(
            CS_INTERFACE,
            &[
                CDC_TYPE_HEADER, // bDescriptorSubtype
                0x10,
                0x01, // bcdCDC (1.10)
            ],
        );
        alt.descriptor(
            CS_INTERFACE,
            &[
                CDC_TYPE_UNION,        // bDescriptorSubtype
                comm_if.into(),        // bControlInterface
                u8::from(comm_if) + 1, // bSubordinateInterface
            ],
        );
        alt.descriptor(
            CS_INTERFACE,
            &[
                CDC_TYPE_ETHERNET, // bDescriptorSubtype
                0x04,              // iMACAddress
                0,                 // bmEthernetStatistics
                0,                 // |
                0,                 // |
                0,                 // |
                0xea,              // wMaxSegmentSize = 1514
                0x05,              // |
                0,                 // wNumberMCFilters
                0,                 // |
                0,                 // bNumberPowerFilters
            ],
        );
        alt.descriptor(
            CS_INTERFACE,
            &[
                CDC_TYPE_NCM, // bDescriptorSubtype
                0x00,         // bcdNCMVersion
                0x01,         // |
                0,            // bmNetworkCapabilities
            ],
        );

        let comm_ep = alt.endpoint_interrupt_in(8, 255);

        // Data interface
        let mut iface = func.interface(Some(data_control));
        let data_if = iface.interface_number();
        let _alt = iface.alt_setting(USB_CLASS_CDC_DATA, 0x00, CDC_PROTOCOL_NTB);
        let mut alt = iface.alt_setting(USB_CLASS_CDC_DATA, 0x00, CDC_PROTOCOL_NTB);
        let read_ep = alt.endpoint_bulk_out(max_packet_size);
        let write_ep = alt.endpoint_bulk_in(max_packet_size);

        CdcNcmClass {
            _comm_if: comm_if,
            comm_ep,
            data_if,
            read_ep,
            write_ep,
            control: control_shared,
        }
    }

    pub fn split(self) -> (Sender<'d, D>, Receiver<'d, D>) {
        (
            Sender {
                write_ep: self.write_ep,
                seq: 0,
            },
            Receiver {
                data_if: self.data_if,
                comm_ep: self.comm_ep,
                read_ep: self.read_ep,

                ntb: [0; NTB_MAX_SIZE],
                ntb_index: 0,
            },
        )
    }
}

pub struct Sender<'d, D: Driver<'d>> {
    write_ep: D::EndpointIn,
    seq: u16,
}

impl<'d, D: Driver<'d>> Sender<'d, D> {
    pub async fn write_packet(&mut self, data: &[u8]) -> Result<(), EndpointError> {
        let seq = self.seq;
        self.seq = self.seq.wrapping_add(1);

        const MAX_PACKET_SIZE: usize = 64; // TODO unhardcode
        const OUT_HEADER_LEN: usize = 28;

        let header = NtbOutHeader {
            nth_sig: NTH_SIG,
            nth_len: 0x0c,
            nth_seq: seq,
            nth_total_len: (data.len() + OUT_HEADER_LEN) as u16,
            nth_first_index: 0x0c,

            ndp_sig: NDP_SIG,
            ndp_len: 0x10,
            ndp_next_index: 0x00,
            ndp_datagram_index: OUT_HEADER_LEN as u16,
            ndp_datagram_len: data.len() as u16,
            ndp_term1: 0x00,
            ndp_term2: 0x00,
        };

        // Build first packet on a buffer, send next packets straight from `data`.
        let mut buf = [0; MAX_PACKET_SIZE];
        let n = byteify(&mut buf, header);
        assert_eq!(n.len(), OUT_HEADER_LEN);

        if OUT_HEADER_LEN + data.len() < MAX_PACKET_SIZE {
            // First packet is not full, just send it.
            // No need to send ZLP because it's short for sure.
            buf[OUT_HEADER_LEN..][..data.len()].copy_from_slice(data);
            self.write_ep
                .write(&buf[..OUT_HEADER_LEN + data.len()])
                .await?;
        } else {
            let (d1, d2) = data.split_at(MAX_PACKET_SIZE - OUT_HEADER_LEN);

            buf[OUT_HEADER_LEN..].copy_from_slice(d1);
            self.write_ep.write(&buf).await?;

            for chunk in d2.chunks(MAX_PACKET_SIZE) {
                self.write_ep.write(&chunk).await?;
            }

            // Send ZLP if needed.
            if d2.len() % MAX_PACKET_SIZE == 0 {
                self.write_ep.write(&[]).await?;
            }
        }

        Ok(())
    }
}

pub struct Receiver<'d, D: Driver<'d>> {
    data_if: InterfaceNumber,
    comm_ep: D::EndpointIn,
    read_ep: D::EndpointOut,

    ntb: [u8; NTB_MAX_SIZE],
    ntb_index: usize,
}

impl<'d, D: Driver<'d>> Receiver<'d, D> {
    /// Reads a single packet from the OUT endpoint.
    pub async fn read_packet(&mut self, buf: &mut [u8]) -> Result<usize, EndpointError> {
        if self.ntb_index == 0 {
            // read NTB
            let mut pos = 0;
            loop {
                let n = self.read_ep.read(&mut self.ntb[pos..]).await?;
                pos += n;
                if n < self.read_ep.info().max_packet_size as usize {
                    break;
                }
            }

            // Process NTB header.
            let sig = u32::from_le_bytes(self.ntb[0..4].try_into().unwrap());
            assert_eq!(sig, NTH_SIG);
            self.ntb_index = u16::from_le_bytes(self.ntb[10..12].try_into().unwrap()) as usize;
            assert_ne!(self.ntb_index, 0);
        }

        let ndp = &self.ntb[self.ntb_index..][..12];
        self.ntb_index = u16::from_le_bytes(ndp[6..8].try_into().unwrap()) as usize;
        let datagram_index = u16::from_le_bytes(ndp[8..10].try_into().unwrap()) as usize;
        let datagram_len = u16::from_le_bytes(ndp[10..12].try_into().unwrap()) as usize;

        buf[..datagram_len].copy_from_slice(&self.ntb[datagram_index..][..datagram_len]);

        Ok(datagram_len)
    }

    /// Waits for the USB host to enable this interface
    pub async fn wait_connection(&mut self) {
        self.read_ep.wait_enabled().await;

        Timer::after(Duration::from_secs(1)).await;

        let buf = [
            0xA1, //bmRequestType
            0x00, //bNotificationType = NETWORK_CONNECTION
            0x01, // wValue = connected
            0x00,
            self.data_if.into(), // wIndex = interface
            0x00,
            0x00, // wLength
            0x00,
        ];
        self.comm_ep.write(&buf).await.unwrap();

        info!("sent notif")
    }
}
