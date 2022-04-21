use crate::pac;
use crate::peripherals::FLASH;
use core::convert::TryInto;
use core::marker::PhantomData;
use core::ptr::write_volatile;
use embassy::util::Unborrow;
use embassy_hal_common::unborrow;

use embedded_storage::nor_flash::{
    ErrorType, MultiwriteNorFlash, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash,
};

const FLASH_SIZE: usize = 0x3FFFF;
const FLASH_BASE: usize = 0x8000000;
const FLASH_START: usize = FLASH_BASE;
const FLASH_END: usize = FLASH_START + FLASH_SIZE;
const PAGE_SIZE: usize = 2048;

pub struct Flash<'d> {
    _inner: FLASH,
    _phantom: PhantomData<&'d mut FLASH>,
}

impl<'d> Flash<'d> {
    pub fn new(p: impl Unborrow<Target = FLASH>) -> Self {
        unborrow!(p);
        Self {
            _inner: p,
            _phantom: PhantomData,
        }
    }

    pub fn unlock(p: impl Unborrow<Target = FLASH>) -> Self {
        let flash = Self::new(p);
        let f = pac::FLASH;
        unsafe {
            f.keyr().write(|w| w.set_keyr(0x4567_0123));
            f.keyr().write(|w| w.set_keyr(0xCDEF_89AB));
        }
        flash
    }

    pub fn lock(&mut self) {
        let f = pac::FLASH;
        unsafe {
            f.cr().modify(|w| w.set_lock(false));
        }
    }

    pub fn blocking_read(&mut self, mut offset: u32, bytes: &mut [u8]) -> Result<(), Error> {
        if offset as usize >= FLASH_END || offset as usize + bytes.len() > FLASH_END {
            return Err(Error::Size);
        }

        let flash_data = unsafe { core::slice::from_raw_parts(offset as *const u8, bytes.len()) };
        bytes.copy_from_slice(flash_data);
        Ok(())
    }

    pub fn blocking_write(&mut self, offset: u32, buf: &[u8]) -> Result<(), Error> {
        if offset as usize + buf.len() > FLASH_END {
            return Err(Error::Size);
        }
        if offset as usize % 8 != 0 || buf.len() as usize % 8 != 0 {
            return Err(Error::Unaligned);
        }

        self.clear_all_err();

        let f = pac::FLASH;
        unsafe {
            f.cr().write(|w| w.set_pg(true));
        }

        let mut ret: Result<(), Error> = Ok(());
        let mut offset = offset;
        for chunk in buf.chunks(8) {
            unsafe {
                write_volatile(
                    offset as *mut u32,
                    u32::from_le_bytes(chunk[0..4].try_into().unwrap()),
                );
                write_volatile(
                    (offset + 4) as *mut u32,
                    u32::from_le_bytes(chunk[4..8].try_into().unwrap()),
                );
            }
            offset += chunk.len() as u32;

            ret = self.blocking_wait_ready();
            if ret.is_err() {
                break;
            }
        }

        unsafe {
            f.cr().write(|w| w.set_pg(false));
        }

        ret
    }

    pub fn blocking_erase(&mut self, from: u32, to: u32) -> Result<(), Error> {
        if to < from || to as usize > FLASH_END {
            return Err(Error::Size);
        }
        if from as usize % PAGE_SIZE != 0 || to as usize % PAGE_SIZE != 0 {
            return Err(Error::Unaligned);
        }

        self.clear_all_err();

        for page in (from..to).step_by(PAGE_SIZE) {
            let f = pac::FLASH;
            let idx = page / PAGE_SIZE as u32;
            unsafe {
                f.cr().modify(|w| {
                    w.set_per(true);
                    w.set_pnb(idx as u8);
                    #[cfg(any(flash_wl55, flash_l0))]
                    w.set_strt(true);
                    #[cfg(any(flash_l4))]
                    w.set_start(true);
                });
            }

            let ret: Result<(), Error> = self.blocking_wait_ready();

            unsafe {
                f.cr().modify(|w| w.set_per(false));
            }

            if ret.is_err() {
                return ret;
            }
        }

        Ok(())
    }

    fn blocking_wait_ready(&self) -> Result<(), Error> {
        loop {
            let f = pac::FLASH;
            let sr = unsafe { f.sr().read() };

            if !sr.bsy() {
                if sr.progerr() {
                    return Err(Error::Prog);
                }

                if sr.wrperr() {
                    return Err(Error::Protected);
                }

                if sr.pgaerr() {
                    return Err(Error::Unaligned);
                }

                if sr.sizerr() {
                    return Err(Error::Size);
                }

                if sr.miserr() {
                    return Err(Error::Miss);
                }

                if sr.pgserr() {
                    return Err(Error::Seq);
                }
                return Ok(());
            }
        }
    }

    fn clear_all_err(&mut self) {
        let f = pac::FLASH;
        unsafe {
            f.sr().write(|w| {
                w.set_rderr(false);
                w.set_fasterr(false);
                w.set_miserr(false);
                w.set_pgserr(false);
                w.set_sizerr(false);
                w.set_pgaerr(false);
                w.set_wrperr(false);
                w.set_progerr(false);
                w.set_operr(false);
                w.set_eop(false);
            });
        }
    }
}

impl Drop for Flash<'_> {
    fn drop(&mut self) {
        self.lock();
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    Prog,
    Size,
    Miss,
    Seq,
    Protected,
    Unaligned,
}

impl<'d> ErrorType for Flash<'d> {
    type Error = Error;
}

impl NorFlashError for Error {
    fn kind(&self) -> NorFlashErrorKind {
        match self {
            Self::Size => NorFlashErrorKind::OutOfBounds,
            Self::Unaligned => NorFlashErrorKind::NotAligned,
            _ => NorFlashErrorKind::Other,
        }
    }
}

impl<'d> ReadNorFlash for Flash<'d> {
    const READ_SIZE: usize = 1;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        self.blocking_read(offset, bytes)
    }

    fn capacity(&self) -> usize {
        todo!()
    }
}

impl<'d> NorFlash for Flash<'d> {
    const WRITE_SIZE: usize = 8;
    const ERASE_SIZE: usize = 2048; // TODO

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        self.blocking_erase(from, to)
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        self.blocking_write(offset, bytes)
    }
}

cfg_if::cfg_if! {
    if #[cfg(feature = "nightly")]
    {
        use embedded_storage_async::nor_flash::{AsyncNorFlash, AsyncReadNorFlash};
        use core::future::Future;

        impl<'d> AsyncNorFlash for Flash<'d> {
            const WRITE_SIZE: usize = <Self as NorFlash>::WRITE_SIZE;
            const ERASE_SIZE: usize = <Self as NorFlash>::ERASE_SIZE;

            type WriteFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a where Self: 'a;
            fn write<'a>(&'a mut self, offset: u32, data: &'a [u8]) -> Self::WriteFuture<'a> {
                async move {
                    todo!()
                }
            }

            type EraseFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a where Self: 'a;
            fn erase<'a>(&'a mut self, from: u32, to: u32) -> Self::EraseFuture<'a> {
                async move {
                    todo!()
                }
            }
        }

        impl<'d> AsyncReadNorFlash for Flash<'d> {
            const READ_SIZE: usize = 4;
            type ReadFuture<'a> = impl Future<Output = Result<(), Self::Error>> + 'a where Self: 'a;
            fn read<'a>(&'a mut self, address: u32, data: &'a mut [u8]) -> Self::ReadFuture<'a> {
                async move {
                    todo!()
                }
            }

            fn capacity(&self) -> usize {
                todo!()
            }
        }
    }
}
