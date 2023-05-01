use crate::driver::{protocol::Error, Interface};
use embedded_hal_async::i2c::Operation;

const ADDRESS: u8 = 0x24;

pub struct I2c<I>(pub I)
where
    I: embedded_hal_async::i2c::I2c;

impl<I> Interface for I2c<I>
where
    I: embedded_hal_async::i2c::I2c,
{
    type Error = I::Error;

    async fn send(&mut self, data: &[u8]) -> Result<(), Error<Self::Error>> {
        self.0.write(ADDRESS, &data).await.map_err(Error::Transport)
    }

    async fn receive(&mut self, buf: &mut [u8]) -> Result<(), Error<Self::Error>> {
        self.0
            .transaction(
                ADDRESS,
                &mut [Operation::Read(&mut [0u8]), Operation::Read(buf)],
            )
            .await
            .map_err(Error::Transport)
    }

    async fn wait_for_ready(&mut self) -> Result<(), Error<Self::Error>> {
        let mut buf = [0u8; 1];
        loop {
            self.0
                .read(ADDRESS, &mut buf)
                .await
                .map_err(Error::Transport)?;

            if buf[0] == 0x01 {
                // we are ready
                break;
            }
        }

        Ok(())
    }
}
