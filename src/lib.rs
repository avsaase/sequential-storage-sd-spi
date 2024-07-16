use block_device_driver::{slice_to_blocks, slice_to_blocks_mut};
use embedded_hal_async::{delay::DelayNs, spi::SpiDevice};
use embedded_storage_async::nor_flash::{
    ErrorType, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash,
};

const BLOCK_SIZE: usize = 512;

#[derive(Debug)]
pub enum Error {
    Sd(sdspi::Error),
}

pub struct SdSpi<SPI, D, ALIGN, const SIZE: usize>
where
    SPI: SpiDevice,
    D: DelayNs,
    ALIGN: aligned::Alignment,
{
    inner: sdspi::SdSpi<SPI, D, ALIGN>,
}

impl<SPI, D, ALIGN, const SIZE: usize> ReadNorFlash for SdSpi<SPI, D, ALIGN, SIZE>
where
    SPI: SpiDevice,
    D: DelayNs + Clone,
    ALIGN: aligned::Alignment,
{
    const READ_SIZE: usize = BLOCK_SIZE;

    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        self.inner
            .read(
                offset / BLOCK_SIZE as u32,
                slice_to_blocks_mut::<ALIGN, BLOCK_SIZE>(bytes),
            )
            .await?;
        Ok(())
    }

    fn capacity(&self) -> usize {
        SIZE
    }
}

impl<SPI, D, ALIGN, const SIZE: usize> NorFlash for SdSpi<SPI, D, ALIGN, SIZE>
where
    SPI: SpiDevice,
    D: DelayNs + Clone,
    ALIGN: aligned::Alignment,
{
    const WRITE_SIZE: usize = BLOCK_SIZE;

    const ERASE_SIZE: usize = BLOCK_SIZE;

    async fn erase(&mut self, _from: u32, _to: u32) -> Result<(), Self::Error> {
        todo!()
    }

    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        self.inner
            .write(
                offset / BLOCK_SIZE as u32,
                slice_to_blocks::<ALIGN, BLOCK_SIZE>(bytes),
            )
            .await?;
        Ok(())
    }
}

impl<SPI, D, ALIGN, const SIZE: usize> ErrorType for SdSpi<SPI, D, ALIGN, SIZE>
where
    SPI: SpiDevice,
    D: DelayNs,
    ALIGN: aligned::Alignment,
{
    type Error = Error;
}

impl NorFlashError for Error {
    fn kind(&self) -> NorFlashErrorKind {
        match self {
            Error::Sd(_) => NorFlashErrorKind::Other,
        }
    }
}

impl From<sdspi::Error> for Error {
    fn from(e: sdspi::Error) -> Self {
        Error::Sd(e)
    }
}
