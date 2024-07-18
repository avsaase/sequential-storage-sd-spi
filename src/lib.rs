#![no_std]

use block_device_adapters::BufStream;
use embedded_hal_async::{delay::DelayNs, spi::SpiDevice};
use embedded_io_async::{Read, Seek, SeekFrom, Write};
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
    D: DelayNs + Clone,
    ALIGN: aligned::Alignment,
{
    // inner: sdspi::SdSpi<SPI, D, ALIGN>,
    // block_address_in_cache: Option<u32>,
    // block_cache: [u8; BLOCK_SIZE],
    card_capacity: u32,
    buf_stream: BufStream<sdspi::SdSpi<SPI, D, ALIGN>, BLOCK_SIZE>,
}

impl<SPI, D, ALIGN> SdSpi<SPI, D, ALIGN>
where
    SPI: SpiDevice,
    D: DelayNs + Clone,
    ALIGN: aligned::Alignment,
{
    pub fn new(device: SPI, delay: D, card_capacity: u32) -> Self {
        Self {
            // inner: sdspi::SdSpi::new(device, delay),
            // block_address_in_cache: None,
            // block_cache: [0u8; BLOCK_SIZE],
            card_capacity,
            buf_stream: BufStream::new(sdspi::SdSpi::new(device, delay)),
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
        self.buf_stream.seek(SeekFrom::Start(offset as u64));
        self.buf_stream.read(bytes).await.unwrap();
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
        todo!("Implement erase using embedded-io-async methods");
    }

    async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        self.buf_stream.seek(SeekFrom::Start(offset as u64));
        self.buf_stream.write(bytes).await.unwrap();
        Ok(())
    }
}

impl<SPI, D, ALIGN> ErrorType for SdSpi<SPI, D, ALIGN>
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
