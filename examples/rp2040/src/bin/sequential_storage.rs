#![no_std]
#![no_main]

use core::ops::Range;

use defmt::{info, warn};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_rp::{
    gpio::{Level, Output},
    spi::{Config, Spi},
};
use embassy_time::{Delay, Timer};
use embedded_hal_bus::spi::ExclusiveDevice;
use embedded_storage_async::nor_flash::{NorFlash, ReadNorFlash};
use embedded_storage_sd_spi::{sd_init, SdSpi};
use panic_probe as _;
use sequential_storage::{
    cache::NoCache,
    erase_all,
    map::{fetch_item, store_item},
};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_rp::init(Default::default());

    let sck = p.PIN_10;
    let mosi = p.PIN_11;
    let miso = p.PIN_12;
    let cs = Output::new(p.PIN_13, Level::High);
    let mut spi_config = Config::default();
    spi_config.frequency = 400_000;

    let mut spi_bus = Spi::new(p.SPI1, sck, mosi, miso, p.DMA_CH0, p.DMA_CH1, spi_config);

    while sd_init(&mut spi_bus).await.is_err() {
        warn!("SD init failed, retrying...");
        Timer::after_millis(10).await;
    }

    let spi_device = ExclusiveDevice::new(spi_bus, cs, Delay);
    let mut sd = SdSpi::<_, _, aligned::A1>::new(spi_device, Delay, 10_240);

    loop {
        match sd.init().await {
            Ok(_) => {
                // spi_config.frequency = 1_000_000;
                // sd.spi().set_config(spi_config);
                info!("SD card initialized");
                break;
            }
            Err(_) => {
                info!("SD init failed, retrying...");
                Timer::after_millis(10).await;
            }
        }
    }

    const FLASH_RANGE: Range<u32> = 10_240..20_480;

    let mut data_buffer = [0u8; 128];

    // info!("Erasing...");
    // erase_all(&mut sd, FLASH_RANGE).await.unwrap();
    // info!("Erased SD card");

    sd.write(0, &[0, 1, 2, 4]).await.unwrap();

    sd.read(0, &mut data_buffer).await.unwrap();
    info!("Read SD card: {}", data_buffer);

    sd.erase(0, 512).await.unwrap();

    sd.read(0, &mut data_buffer).await.unwrap();
    info!("Read SD card: {}", data_buffer);

    sd.write(10, &[0, 1, 2, 4]).await.unwrap();

    sd.read(0, &mut data_buffer).await.unwrap();
    info!("Read SD card: {}", data_buffer);

    // info!("Storing item...");
    // store_item(
    //     &mut sd,
    //     FLASH_RANGE,
    //     &mut NoCache::new(),
    //     &mut data_buffer,
    //     &1,
    //     &1234,
    // )
    // .await;
    // sd.read(10_240, &mut data_buffer).await.unwrap();
    // info!("Read SD card: {}", data_buffer);
    // info!("Stored item");

    // info!("Fetching item...");
    // let retrieved_item: Option<u32> = fetch_item(
    //     &mut sd,
    //     FLASH_RANGE,
    //     &mut NoCache::new(),
    //     &mut data_buffer,
    //     &1,
    // )
    // .await
    // .unwrap();

    // match retrieved_item {
    //     Some(item) => info!("Retrieved item: {}", item),
    //     None => info!("Item not found"),
    // }
}
