use defmt::{debug, write, Format, Formatter};

// some code from: https://github.com/WMT-GmbH/pn532/blob/master/src/protocol.rs

const ACK: [u8; 6] = [0x00, 0x00, 0xFF, 0x00, 0xFF, 0x00];

pub enum Error<E> {
    TooMuchData,
    Transport(E),
    NotAcknowledged,
    BadResponse,
    BadChecksum,
    BufferUnderflow,
    Syntax,
}

impl<E> Format for Error<E> {
    fn format(&self, fmt: Formatter) {
        match self {
            Self::TooMuchData => write!(fmt, "Too much data"),
            Self::Transport(_err) => write!(fmt, "Transport error"),
            Self::NotAcknowledged => write!(fmt, "Not acknowledged"),
            Self::BadResponse => write!(fmt, "Bad response"),
            Self::BadChecksum => write!(fmt, "Bad checksum"),
            Self::BufferUnderflow => write!(fmt, "Buffer underflow"),
            Self::Syntax => write!(fmt, "Syntax error"),
        }
    }
}

const HOST_TO_PN532: u8 = 0xD4;
const PN532_TO_HOST: u8 = 0xD5;

pub trait Interface {
    type Error;

    async fn send(&mut self, request: &[u8]) -> Result<(), Error<Self::Error>>;
    async fn receive(&mut self, buf: &mut [u8]) -> Result<(), Error<Self::Error>>;
    async fn wait_for_ready(&mut self) -> Result<(), Error<Self::Error>>;
}

pub struct Protocol<I: Interface, const B: usize = 255> {
    buffer: [u8; B],
    interface: I,
}

impl<I: Interface, const B: usize> Protocol<I, B> {
    pub fn new(interface: I) -> Self {
        Self {
            buffer: [0u8; B],
            interface,
        }
    }

    /// send a simple command
    #[allow(unused)]
    pub async fn send(&mut self, cmd: u8, data: &[u8]) -> Result<(), Error<I::Error>> {
        self.send_request(cmd, data).await?;
        self.wait_for_ready().await?;
        self.read_ack().await?;
        Ok(())
    }

    /// send a request/response command
    pub async fn request(
        &mut self,
        cmd: u8,
        data: &[u8],
        response_len: u8,
    ) -> Result<(u8, &[u8]), Error<I::Error>> {
        debug!("Sending request");
        self.send_request(cmd, data).await?;
        debug!("Waiting for ack");
        self.wait_for_ready().await?;
        debug!("Reading ack");
        self.read_ack().await?;
        debug!("Waiting for response");
        self.wait_for_ready().await?;
        debug!("Read response");
        self.read_response(response_len).await
    }

    async fn send_request(&mut self, cmd: u8, data: &[u8]) -> Result<(), Error<I::Error>> {
        let data_len = data.len();
        if data_len > B - 10 {
            return Err(Error::TooMuchData);
        }
        let frame_len = 2 + data_len as u8;

        let mut data_sum = HOST_TO_PN532.wrapping_add(cmd);
        for b in data {
            data_sum = data_sum.wrapping_add(*b);
        }

        const fn to_checksum(sum: u8) -> u8 {
            (!sum).wrapping_add(1)
        }

        // PREAMBLE
        self.buffer[0] = 0x00;
        // START CODE
        self.buffer[1] = 0x00;
        self.buffer[2] = 0xFF;
        // LEN
        self.buffer[3] = frame_len as u8;
        // LCS
        self.buffer[4] = to_checksum(frame_len);
        // TFI
        self.buffer[5] = HOST_TO_PN532;
        // DATA
        self.buffer[6] = cmd;
        self.buffer[7..7 + data_len].copy_from_slice(data);
        // DCS
        self.buffer[7 + data_len] = to_checksum(data_sum);
        // POSTAMBLE
        self.buffer[8 + data_len] = 0x00;

        self.interface.send(&self.buffer[..9 + data_len]).await?;

        Ok(())
    }

    async fn wait_for_ready(&mut self) -> Result<(), Error<I::Error>> {
        // FIXME: wait for external interrupt

        self.interface.wait_for_ready().await?;

        Ok(())
    }

    async fn read_ack(&mut self) -> Result<(), Error<I::Error>> {
        let mut buf = [0u8; 6];

        self.interface.receive(&mut buf).await?;

        // FIXME: scan for start

        if buf != ACK {
            debug!("Not acked: {:?}", buf);
            Err(Error::NotAcknowledged)
        } else {
            Ok(())
        }
    }

    async fn read_response(&mut self, len: u8) -> Result<(u8, &[u8]), Error<I::Error>> {
        let buf = &mut self.buffer[0..len as usize + 9];
        buf.fill(0);

        self.interface.receive(buf).await?;

        // FIXME: scan for start

        Self::process_response(buf)
    }

    fn process_response(buf: &[u8]) -> Result<(u8, &[u8]), Error<I::Error>> {
        if buf[0..3] != [0x00, 0x00, 0xFF] {
            return Err(Error::BadResponse);
        }
        // Check length & length checksum
        let frame_len = buf[3];
        if (frame_len.wrapping_add(buf[4])) != 0 {
            return Err(Error::BadChecksum);
        }
        if frame_len == 0 {
            return Err(Error::BadResponse);
        }
        if frame_len == 1 {
            // 6.2.1.5 Error frame
            return Err(Error::Syntax);
        }
        match buf.get(5 + frame_len as usize + 1) {
            None => {
                return Err(Error::BufferUnderflow);
            }
            Some(&0x00) => {}
            Some(_) => {
                return Err(Error::BadResponse);
            }
        }

        if buf[5] != PN532_TO_HOST {
            return Err(Error::BadResponse);
        }

        // Check frame checksum value matches bytes
        let checksum = buf[5..5 + frame_len as usize + 1]
            .iter()
            .fold(0u8, |s, &b| s.wrapping_add(b));

        if checksum != 0 {
            return Err(Error::BadChecksum);
        }

        // Adjust response buf and return it
        Ok((buf[6], &buf[7..5 + frame_len as usize]))
    }
}
