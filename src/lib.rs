#![no_std]

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

pub struct SdSpi<SPI, D, ALIGN>
where
    SPI: SpiDevice,
    D: DelayNs,
    ALIGN: aligned::Alignment,
{
    inner: sdspi::SdSpi<SPI, D, ALIGN>,
    block_address_in_cache: Option<u32>,
    block_cache: [u8; BLOCK_SIZE],
    card_capacity: u32,
}

impl<SPI, D, ALIGN> SdSpi<SPI, D, ALIGN>
where
    SPI: SpiDevice,
    D: DelayNs + Clone,
    ALIGN: aligned::Alignment,
{
    pub fn new(device: SPI, delay: D, card_capacity: u32) -> Self {
        Self {
            inner: sdspi::SdSpi::new(device, delay),
            block_address_in_cache: None,
            block_cache: [0u8; BLOCK_SIZE],
            card_capacity,
        }
    }

    pub async fn init(&mut self) -> Result<(), Error> {
        self.inner.init().await?;
        Ok(())
    }

    pub fn spi(&mut self) -> &mut SPI {
        self.inner.spi()
    }
}

impl<SPI, D, ALIGN> ReadNorFlash for SdSpi<SPI, D, ALIGN>
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
                    &mut slice_to_blocks_mut::<ALIGN, BLOCK_SIZE>(&mut self.block_cache),
                )
                .await?;
            self.block_address_in_cache = Some(block_address);
        }

        let offset_in_block = offset as usize % BLOCK_SIZE;
        let length = bytes.len();
        bytes.copy_from_slice(&self.block_cache[offset_in_block..(offset_in_block + length)]);
        Ok(())
    }

    fn capacity(&self) -> usize {
        self.card_capacity as usize
    }
}

impl<SPI, D, ALIGN> NorFlash for SdSpi<SPI, D, ALIGN>
where
    SPI: SpiDevice,
    D: DelayNs + Clone,
    ALIGN: aligned::Alignment,
{
    const WRITE_SIZE: usize = 1;

    const ERASE_SIZE: usize = BLOCK_SIZE;

    async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        let from_block_address = address_to_block(from);
        let to_block_address = address_to_block(to);

        for block_address in from_block_address..=to_block_address {
            self.inner
                .write(
                    block_address,
                    slice_to_blocks::<ALIGN, BLOCK_SIZE>(&[0u8; BLOCK_SIZE]),
                )
                .await?;
        }
        self.block_address_in_cache = None;
        Ok(())
    }

    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        let block_address = address_to_block(offset);

        // Read block into cache if needed
        if self.block_address_in_cache != Some(block_address) {
            self.inner
                .read(
                    block_address,
                    &mut slice_to_blocks_mut::<ALIGN, BLOCK_SIZE>(&mut self.block_cache),
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
                &mut slice_to_blocks::<ALIGN, BLOCK_SIZE>(&self.block_cache),
            )
            .await?;

        Ok(())
    }
}

impl<SPI, D, ALIGN> ErrorType for SdSpi<SPI, D, ALIGN>
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

fn address_to_block(address: u32) -> u32 {
    address / BLOCK_SIZE as u32
}
