use crate::driver::{protocol::Error, Interface};
use defmt::debug;
use embedded_hal_async::spi::Operation;

// some code from: https://github.com/WMT-GmbH/pn532/blob/master/src/spi.rs

const fn as_lsb(byte: u8) -> u8 {
    byte
}

pub const PN532_SPI_STATREAD: u8 = as_lsb(0x02);
pub const PN532_SPI_DATAWRITE: u8 = as_lsb(0x01);
pub const PN532_SPI_DATAREAD: u8 = as_lsb(0x03);
/// ready indicator
pub const PN532_SPI_READY: u8 = as_lsb(0x01);

pub struct Spi<I>(pub I)
where
    I: embedded_hal_async::spi::SpiDevice;

impl<I> Interface for Spi<I>
where
    I: embedded_hal_async::spi::SpiDevice,
{
    type Error = I::Error;

    async fn send(&mut self, request: &[u8]) -> Result<(), Error<Self::Error>> {
        let len = request.len();
        let mut buf = [0u8; 255];
        buf[0] = PN532_SPI_DATAWRITE;
        buf[1..len + 1].copy_from_slice(request);

        debug!("Request: {}", &buf[0..len + 1]);

        self.0
            //    .write_transaction(&[&[PN532_SPI_DATAWRITE], request])
            .write(&buf[0..len + 1])
            .await
            .map_err(Error::Transport)?;

        Ok(())
    }

    async fn receive(&mut self, buf: &mut [u8]) -> Result<(), Error<Self::Error>> {
        // workaround due to https://github.com/embassy-rs/embassy/issues/1411
        /*
                buf[0] = PN532_SPI_DATAREAD;
                self.0
                    .transfer_in_place(buf)
                    // .transfer(..)
                    .await
                    .map_err(Error::Transport)?;
        */

        self.0
            .transaction(&mut [
                Operation::Write(&[PN532_SPI_DATAREAD]),
                Operation::Read(buf),
            ])
            .await
            .map_err(Error::Transport)?;

        Ok(())
    }

    async fn wait_for_ready(&mut self) -> Result<(), Error<Self::Error>> {
        let mut buf = [0u8; 1];

        let mut cnt = 0usize;

        loop {
            cnt += 1;

            /*
            self.0
                .transfer(&mut buf, &[PN532_SPI_STATREAD])
                .await
                .map_err(Error::Transport)?;
             */
            self.0
                .transaction(&mut [
                    Operation::Write(&[PN532_SPI_STATREAD]),
                    Operation::Read(&mut buf),
                ])
                .await
                .map_err(Error::Transport)?;

            if buf[0] == PN532_SPI_READY {
                // we are ready
                break;
            }

            //debug!("State: {}", buf);

            // Timer::after(Duration::from_millis(100)).await;
        }

        debug!("Ready after {0} checks", cnt);

        Ok(())
    }
}
