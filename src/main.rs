#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

// extern crate alloc;

use crate::driver::protocol::Interface;
use crate::driver::requests::{CardType, SAMMode};
use crate::driver::Reader;
use defmt::{write, *};
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::spi::{BitOrder, Config, Spi};
use embassy_stm32::time::Hertz;
use embassy_time::{Duration, Timer};
use embedded_hal_async::spi::ExclusiveDevice;
use {defmt_rtt as _, panic_probe as _};

mod driver;
mod ndef;
mod reader;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_stm32::init(Default::default());

    /*
    let irq = interrupt::take!(I2C1_EV);
    let i2c = I2c::new(
        p.I2C1,
        p.PB6,
        p.PB7,
        irq,
        NoDma, //p.DMA1_CH6,
        NoDma, //p.DMA1_CH7,
        Hertz(100_000),
        Default::default(),
    );*/

    // let reader = Pn532::new_async(I2CInterface { i2c });

    /*
    let mut spi = Spi::new(
        p.SPI3,
        p.PB3,
        p.PB5,
        p.PB4,
        p.DMA2_CH2,
        p.DMA2_CH1,
        Hertz(100_000),
        {
            let mut config = Config::default();
            config.bit_order = BitOrder::LsbFirst;
            config
        },
    );*/

    let mut spi = Spi::new(
        p.SPI1,
        p.PA5,
        p.PA12,
        p.PA11,
        p.DMA1_CH3,
        p.DMA1_CH2,
        Hertz(100_000),
        {
            let mut config = Config::default();
            config.bit_order = BitOrder::LsbFirst;
            config
        },
    );

    // SPI3
    // let cs = Output::new(p.PA4, Level::High, Speed::VeryHigh);
    // SPI1
    let mut cs = Output::new(p.PB0, Level::High, Speed::VeryHigh);

    // reset

    info!("Performing reset ...");
    let mut rst = Output::new(p.PA8, Level::High, Speed::High);
    Timer::after(Duration::from_millis(100)).await;
    rst.set_low();
    Timer::after(Duration::from_millis(500)).await;
    rst.set_high();
    Timer::after(Duration::from_millis(100)).await;
    info!("Performing reset ... done!");

    // wakeup

    info!("Sending wakup");
    Timer::after(Duration::from_millis(1000)).await;
    unwrap!(spi.write(&[0u8]).await);

    // go

    info!("Run");
    cs.set_low();
    Timer::after(Duration::from_millis(100)).await;

    let device = ExclusiveDevice::new(spi, cs);
    let mut reader = Reader::new(driver::Spi(device));

    let response = unwrap!(reader.get_firmware_version().await);
    info!("Firmware: {}", response);

    unwrap!(reader.sam_configuration(SAMMode::Normal, false).await);

    loop {
        let result = reader.read_passive_target(CardType::IsoTypeA).await;
        if let Ok(Some(card)) = result {
            info!("Card: {}", card);

            match read_key(&mut reader).await {
                Ok(key) => {
                    info!("Key: {}", key);
                }
                Err(err) => {
                    info!("Key read failed: {}", err);
                }
            }

            Timer::after(Duration::from_secs(2)).await;
        }
    }
}

pub struct Key([u8; 8]);

impl Format for Key {
    fn format(&self, fmt: Formatter) {
        defmt::write!(fmt, "{:X}", self.0);
    }
}

pub enum ReadKeyError<I: Interface> {
    Io(driver::ReadError<I::Error>),
    Ndef(ndef::Error),
}

impl<I: Interface> Format for ReadKeyError<I> {
    fn format(&self, fmt: Formatter) {
        match self {
            Self::Io(err) => write!(fmt, "I/O error: {}", err),
            Self::Ndef(err) => write!(fmt, "NDEF error: {}", err),
        }
    }
}

impl<I: Interface> From<driver::ReadError<I::Error>> for ReadKeyError<I> {
    fn from(value: driver::ReadError<I::Error>) -> Self {
        Self::Io(value)
    }
}

impl<I: Interface> From<ndef::Error> for ReadKeyError<I> {
    fn from(value: ndef::Error) -> Self {
        Self::Ndef(value)
    }
}

async fn read_key<I: Interface>(reader: &mut Reader<I>) -> Result<Option<Key>, ReadKeyError<I>> {
    let read = reader.read_ntag(0).await?;

    trace!("Read 0: {:X}", read[0..4]);
    trace!("Read 1: {:X}", read[4..8]);
    trace!("Read 2: {:X}", read[8..12]);
    trace!("Read 3: {:X}", read[12..16]);

    let max = read[12 + 2] as u16 * 8;
    info!(
        "Max size: {} ({} pages, {} chunks)",
        max,
        max / 4,
        max / (4 * 4)
    );

    if max < 1024 {
        let mut buf = [0u8; 1024];

        let max_p = max / 4;
        let mut p = 4u8; // start page
        let mut i = 0;

        while (p as u16) <= max_p {
            debug!("Read page starting: {}", p);
            let read = reader.read_ntag(p).await?;

            buf[i..i + 16].copy_from_slice(&read);

            // advance index by 16 bytes
            i += 16;

            // advance by 4 pages (4 bytes each)
            p += 4;
        }

        let data = &buf[0..max as usize];

        info!("NDEF: {:02X}", data);

        for record in ndef::Reader::new(&data) {
            let record = record?;
            info!("{:X}", record);
        }
    }

    Ok(None)
}
