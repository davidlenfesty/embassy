use crate::peripherals::FLASH;
use core::marker::PhantomData;
use embassy::util::Unborrow;
use embassy_hal_common::unborrow;

use embedded_storage::nor_flash::{
    ErrorType, MultiwriteNorFlash, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash,
};

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
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    OutOfBounds,
    Unaligned,
}

impl<'d> ErrorType for Flash<'d> {
    type Error = Error;
}

impl NorFlashError for Error {
    fn kind(&self) -> NorFlashErrorKind {
        match self {
            Self::OutOfBounds => NorFlashErrorKind::OutOfBounds,
            Self::Unaligned => NorFlashErrorKind::NotAligned,
        }
    }
}

impl<'d> ReadNorFlash for Flash<'d> {
    const READ_SIZE: usize = 1;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        todo!()
    }

    fn capacity(&self) -> usize {
        todo!()
    }
}

impl<'d> NorFlash for Flash<'d> {
    const WRITE_SIZE: usize = 4;
    const ERASE_SIZE: usize = 2048; // TODO

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        todo!()
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        todo!()
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
