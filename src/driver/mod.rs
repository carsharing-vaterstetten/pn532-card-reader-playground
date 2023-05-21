use defmt::{debug, trace, write, Format, Formatter};

mod i2c;
pub mod protocol;
pub mod requests;
mod spi;

use crate::driver::protocol::{Interface, Protocol};
use crate::driver::requests::{CardType, Command, NTAGCommand, SAMMode};
pub use i2c::I2c;
pub use spi::Spi;

pub enum Error<E> {
    Protocol(protocol::Error<E>),
    InvalidResponse,
    Decoder,
}

impl<E> Format for Error<E> {
    fn format(&self, fmt: Formatter) {
        match self {
            Self::Protocol(err) => write!(fmt, "Protocol error: {}", err),
            Self::InvalidResponse => write!(fmt, "Invalid response"),
            Self::Decoder => write!(fmt, "Decoder error"),
        }
    }
}

pub enum ReadError<E> {
    Reader(Error<E>),
    ReadError,
}

impl<E> From<Error<E>> for ReadError<E> {
    fn from(value: Error<E>) -> Self {
        Self::Reader(value)
    }
}

impl<E> Format for ReadError<E> {
    fn format(&self, fmt: Formatter) {
        match self {
            Self::Reader(err) => write!(fmt, "Protocol error: {}", err),
            Self::ReadError => write!(fmt, "Read error"),
        }
    }
}

pub struct Reader<I>
where
    I: Interface,
{
    protocol: Protocol<I, 200>,
}

impl<I> Reader<I>
where
    I: Interface,
{
    pub fn new(interface: I) -> Self {
        Self {
            protocol: Protocol::new(interface),
        }
    }

    pub async fn get_firmware_version(&mut self) -> Result<FirmwareVersion, Error<I::Error>> {
        self.request(Request::<0>::get_firmware_version().borrow())
            .await
    }

    pub async fn sam_configuration(
        &mut self,
        mode: SAMMode,
        use_irq: bool,
    ) -> Result<(), Error<I::Error>> {
        self.request(Request::<0>::sam_configuration(mode, use_irq).borrow())
            .await
    }

    pub async fn read_ntag(&mut self, page: u8) -> Result<[u8; 16], ReadError<I::Error>> {
        self.request(Request::<0>::ntag_read(page).borrow())
            .await
            .map_err(ReadError::Reader)
            .and_then(|d| match d {
                DataReadResult::Err => Err(ReadError::ReadError),
                DataReadResult::Ok(data) => Ok(data),
            })
    }

    pub async fn read_passive_target(
        &mut self,
        card_type: CardType,
    ) -> Result<Option<CardUid>, Error<I::Error>> {
        self.request(Request::<0>::in_list_passive_target(card_type).borrow())
            .await
    }

    pub async fn request<D>(&mut self, request: BorrowedRequest<'_>) -> Result<D, Error<I::Error>>
    where
        D: Decode,
    {
        let (response, data) = self
            .protocol
            .request(request.command as u8, &request.data, D::LEN as u8)
            .await
            .map_err(Error::Protocol)?;

        if response != request.command as u8 + 1 {
            debug!("Response: {}", response);
            return Err(Error::InvalidResponse);
        }

        Ok(D::decode(data).map_err(|_| Error::Decoder)?)
    }
}

#[derive(Clone, Debug, Format)]
pub struct FirmwareVersion {
    pub ic: u8,
    pub version: u8,
    pub revision: u8,
    pub supports_iso18092: bool,
    pub supports_iso14443_a: bool,
    pub supports_iso14443_b: bool,
}

pub enum DataReadResult<const N: usize> {
    Ok([u8; N]),
    Err,
}

impl<const N: usize> Decode for DataReadResult<N> {
    type Error = ();
    const LEN: usize = N + 1;

    fn decode(data: &[u8]) -> Result<Self, Self::Error> {
        trace!("DRR: len {} - expected: {}", data.len(), Self::LEN);
        trace!("DRR: data {:#X}", data);

        if data.len() != Self::LEN {
            return Err(());
        }

        if data[0] != 0x00 {
            return Ok(Self::Err);
        }

        let mut result = [0u8; N];
        result.copy_from_slice(&data[1..]);
        Ok(Self::Ok(result))
    }
}

pub trait Decode: Sized {
    type Error;
    const LEN: usize;

    fn decode(data: &[u8]) -> Result<Self, Self::Error>;
}

impl Decode for () {
    type Error = ();
    const LEN: usize = 0;

    fn decode(_data: &[u8]) -> Result<Self, Self::Error> {
        Ok(())
    }
}

impl<const N: usize> Decode for [u8; N] {
    type Error = ();
    const LEN: usize = N;

    fn decode(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() != Self::LEN {
            return Err(());
        }

        let mut result = [0u8; N];
        result.copy_from_slice(data);
        Ok(result)
    }
}

impl Decode for FirmwareVersion {
    type Error = ();

    const LEN: usize = 4;

    fn decode(data: &[u8]) -> Result<Self, Self::Error> {
        if data.len() != Self::LEN {
            return Err(());
        }

        Ok(Self {
            ic: data[0],
            version: data[1],
            revision: data[2],
            supports_iso18092: data[3] & 0x04 > 0,
            supports_iso14443_b: data[3] & 0x02 > 0,
            supports_iso14443_a: data[3] & 0x01 > 0,
        })
    }
}

impl Decode for Option<CardUid> {
    type Error = ();
    const LEN: usize = 19;

    fn decode(data: &[u8]) -> Result<Self, Self::Error> {
        if data[0] != 1 {
            // can only handle a single card
            return Ok(None);
        }
        let len = data[5] as usize;
        if len > 7 {
            // UID too long
            return Ok(None);
        }

        let mut uid = [0u8; 7];
        uid[..len].copy_from_slice(&data[6..6 + len]);

        Ok(Some(CardUid(uid)))
    }
}

pub struct CardUid(pub [u8; 7]);

impl Format for CardUid {
    fn format(&self, fmt: Formatter) {
        write!(fmt, "{:X}", self.0)
    }
}

pub struct Request<const N: usize> {
    pub command: Command,
    pub data: [u8; N],
}

impl<const N: usize> Request<N> {
    pub const fn new(command: Command, data: [u8; N]) -> Request<N> {
        Self { command, data }
    }

    pub const fn get_firmware_version() -> Request<0> {
        Request::<0>::new(Command::GetFirmwareVersion, [])
    }

    pub const fn sam_configuration(mode: SAMMode, use_irq: bool) -> Request<3> {
        let (mode, timeout) = match mode {
            SAMMode::Normal => (1, 0),
            SAMMode::VirtualCard { timeout } => (2, timeout),
            SAMMode::WiredCard => (3, 0),
            SAMMode::DualCard => (4, 0),
        };
        Request::<3>::new(Command::SAMConfiguration, [mode, timeout, use_irq as u8])
    }

    pub const fn ntag_read(page: u8) -> Request<3> {
        Request::new(
            Command::InDataExchange,
            [0x01 /* target (tg) */, NTAGCommand::Read as u8, page],
        )
    }

    pub const fn in_list_passive_target(card_type: CardType) -> Request<2> {
        const MAX_CARDS: u8 = 0x01;
        Request::new(Command::InListPassiveTarget, [MAX_CARDS, card_type as u8])
    }

    pub(crate) fn borrow(&self) -> BorrowedRequest<'_> {
        BorrowedRequest {
            command: self.command,
            data: &self.data,
        }
    }
}

pub struct BorrowedRequest<'a> {
    pub command: Command,
    pub data: &'a [u8],
}
