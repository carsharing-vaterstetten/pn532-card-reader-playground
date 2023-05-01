#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]
#![feature(async_fn_in_trait)]
#![allow(incomplete_features)]

use crate::driver::requests::{CardType, SAMMode};
use crate::driver::Reader;
use defmt::*;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::spi::{BitOrder, Config, Spi};
use embassy_stm32::time::Hertz;
use embassy_time::{Duration, Timer};
use embedded_hal_async::spi::ExclusiveDevice;
use {defmt_rtt as _, panic_probe as _};

mod driver;

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
        }
    }
}
