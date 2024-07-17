use block_device_driver::{slice_to_blocks, slice_to_blocks_mut};
use embedded_hal_async::{delay::DelayNs, spi::SpiDevice};
use embedded_storage_async::nor_flash::{
    ErrorType, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash,
};

pub use sdspi::sd_init;

const BLOCK_SIZE: usize = 512;

#[derive(Debug)]
pub enum Error {
    Sd(sdspi::Error),
}

pub struct SdSpi<SPI, D, ALIGN, const SIZE: usize>
where
    SPI: SpiDevice,
    D: DelayNs + Clone,
    ALIGN: aligned::Alignment,
{
    inner: sdspi::SdSpi<SPI, D, ALIGN>,
    block_address_in_cache: Option<u32>,
    block_cache: [u8; BLOCK_SIZE],
}

impl<SPI, D, ALIGN, const SIZE: usize> SdSpi<SPI, D, ALIGN, SIZE>
where
    SPI: SpiDevice,
    D: DelayNs + Clone,
    ALIGN: aligned::Alignment,
{
    pub fn new(device: SPI, delay: D) -> Self {
        Self {
            inner: sdspi::SdSpi::new(device, delay),
            block_address_in_cache: None,
            block_cache: [0u8; BLOCK_SIZE],
        }
    }
}

impl<SPI, D, ALIGN, const SIZE: usize> ReadNorFlash for SdSpi<SPI, D, ALIGN, SIZE>
where
    SPI: SpiDevice,
    D: DelayNs + Clone,
    ALIGN: aligned::Alignment,
{
    const READ_SIZE: usize = 1;

    async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let block_address = address_to_block(offset);

        // Read block into cache if needed
        if self.block_address_in_cache != Some(block_address) {
            self.inner
                .read(
                    block_address,
                    &mut slice_to_blocks_mut::<ALIGN, SIZE>(&mut self.block_cache),
                )
                .await?;
            self.block_address_in_cache = Some(block_address);
        }

        bytes.copy_from_slice(&self.block_cache[..bytes.len()]);
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
    const WRITE_SIZE: usize = 1;

    const ERASE_SIZE: usize = BLOCK_SIZE;

    async fn erase(&mut self, _from: u32, _to: u32) -> Result<(), Self::Error> {
        todo!()
    }

    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        let block_address = address_to_block(offset);

        // Read block into cache if needed
        if self.block_address_in_cache != Some(block_address) {
            self.inner
                .read(
                    block_address,
                    &mut slice_to_blocks_mut::<ALIGN, SIZE>(&mut self.block_cache),
                )
                .await?;
            self.block_address_in_cache = Some(block_address);
        }

        // Update the cache
        let offset_in_block = offset % BLOCK_SIZE as u32;
        let length = bytes.len();
        self.block_cache[(offset_in_block as usize)..(offset_in_block as usize + length)]
            .copy_from_slice(bytes);

        // Write the cache back to the card
        self.inner
            .write(
                block_address,
                &mut slice_to_blocks::<ALIGN, SIZE>(&self.block_cache),
            )
            .await?;
        Ok(())
    }
}

impl<SPI, D, ALIGN, const SIZE: usize> ErrorType for SdSpi<SPI, D, ALIGN, SIZE>
where
    SPI: SpiDevice,
    D: DelayNs + Clone,
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

fn address_to_block(address: u32) -> u32 {
    address / BLOCK_SIZE as u32
}
