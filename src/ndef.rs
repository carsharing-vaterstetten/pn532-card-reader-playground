use core::str::from_utf8;
use defmt::{debug, trace, write, Format, Formatter};

#[derive(Format)]
pub enum Record<'d> {
    // Empty (0x00)
    Empty,
    // Well-known (0x01)
    WellKnown,
    // Mime Media (0x02)
    MimeMedia { r#type: &'d str, value: &'d str },
    // MimeMedia { r#type: &'d [u8], value: &'d [u8] },
    AbsoluteUri,
    External,
    Unknown,
    Unchanged,
    Unexpected(&'d [u8]),
}

pub struct Reader<'d> {
    data: &'d [u8],
}

impl<'d> Reader<'d> {
    pub fn new(data: &'d [u8]) -> Self {
        Self { data }
    }
}

impl<'d> IntoIterator for Reader<'d> {
    type Item = Result<Record<'d>, Error>;
    type IntoIter = ReaderIter<'d>;

    fn into_iter(self) -> Self::IntoIter {
        ReaderIter {
            data: &self.data,
            position: 0,
            state: IterState::Fresh,
        }
    }
}

enum IterState {
    Fresh,
    Reading,
    Complete,
}

pub struct ReaderIter<'d> {
    data: &'d [u8],
    position: usize,
    state: IterState,
}

impl<'d> ReaderIter<'d> {
    fn is_unformatted(&self) -> bool {
        return self.data[0] == 0xFF
            && self.data[1] == 0xFF
            && self.data[2] == 0xFF
            && self.data[3] == 0xFF;
    }

    fn try_next(&mut self) -> Result<Option<Record<'d>>, Error> {
        // check if we read through all the data

        trace!("position = {}, len: {}", self.position, self.data.len());

        match self.state {
            IterState::Complete => {
                return Ok(None);
            }
            IterState::Reading => {}
            IterState::Fresh => {
                if self.is_unformatted() {
                    // we keep the state in order to run into this again, and again, …
                    return Err(Error::NotFormatted);
                }
                if self.data.len() < 8 {
                    return Err(Error::UnderflowHeader);
                }

                if self.data[0] == 0x03 {
                    debug!("Message len: {}, start: 2", self.data[1]);
                    self.position = 2;
                } else if self.data[5] == 0x03 {
                    debug!("Message len: {}, start: 7", self.data[6]);
                    self.position = 7;
                }
                self.state = IterState::Reading;
            }
        };

        trace!("flags - tnf: {0=0..3}, il: {0=3..4}, sr: {0=4..5}, chunk: {0=5..6}, me: {0=6..7}, mb: {0=7..8}", self.data[self.position]);
        let flags = RecordHeaderFlags(self.data[self.position]);
        debug!("flags: {}", flags);

        let mut header_len = 1 // flags
            + 1; // type length

        if !flags.is_short_record() {
            header_len += 1;
        }
        match flags.is_short_record() {
            true => header_len += 1,
            false => header_len += 4,
        };

        if flags.has_id_length() {
            header_len += 1
        }

        if self.data.len() - self.position < header_len {
            return Err(Error::UnderflowHeader);
        }

        let mut idx = 1;

        let type_len = self.data[self.position + idx] as usize;
        idx += 1;

        let payload_len = match flags.is_short_record() {
            true => {
                let len = self.data[self.position + idx] as usize;
                idx += 1;
                len
            }
            false => {
                debug!("Position: {}", self.position + idx);
                let len = u32::from_be_bytes([
                    self.data[self.position + idx],
                    self.data[self.position + idx + 1],
                    self.data[self.position + idx + 2],
                    self.data[self.position + idx + 3],
                ]);
                idx += 4;
                len as usize
            }
        };
        let id_len = match flags.has_id_length() {
            true => {
                let len = self.data[self.position + idx];
                idx += 1;
                len as usize
            }
            false => 0,
        };

        debug!(
            "header: {}, type: {}, payload: {}, id: {}",
            header_len, type_len, payload_len, id_len
        );

        let total_len = header_len + type_len + payload_len + id_len;
        if self.data.len() - self.position < total_len {
            return Err(Error::UnderflowPayload);
        }

        let start = self.position + header_len;
        let result = match flags.tnf() {
            0x00 => Record::Empty,
            0x02 => {
                let r#type = &self.data[start..start + type_len];
                let value =
                    &self.data[start + type_len + id_len..start + type_len + id_len + payload_len];
                trace!("type: {:X}, value: {:X}", r#type, value);
                Record::MimeMedia {
                    r#type: from_utf8(r#type)?,
                    value: from_utf8(value)?,
                    // r#type,
                    // value,
                }
            }
            0x05 => Record::Unknown,
            _ => Record::Unexpected(&self.data[self.position..self.position + total_len]),
        };

        self.position += total_len;

        if flags.message_end() {
            self.state = IterState::Complete;
        }

        Ok(Some(result))
    }
}

impl<'d> Iterator for ReaderIter<'d> {
    type Item = Result<Record<'d>, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        self.try_next().transpose()
    }
}

#[derive(Format)]
pub enum Error {
    NotFormatted,
    UnderflowHeader,
    UnderflowPayload,
    Utf8,
}

impl From<core::str::Utf8Error> for Error {
    fn from(_value: core::str::Utf8Error) -> Self {
        Self::Utf8
    }
}

pub struct RecordHeaderFlags(u8);

impl Format for RecordHeaderFlags {
    fn format(&self, fmt: Formatter) {
        write!(
            fmt,
            "Flags(tnf: {:x}, mb: {}, me: {}, chunk: {}, shortRecord: {}, idLength: {})",
            self.tnf(),
            self.message_begin(),
            self.message_end(),
            self.chunk(),
            self.is_short_record(),
            self.has_id_length(),
        );
    }
}

impl RecordHeaderFlags {
    pub fn tnf(&self) -> u8 {
        self.0 & 0b0000_0111
    }

    pub fn has_id_length(&self) -> bool {
        self.0 & 0b0000_1000 > 0
    }

    pub fn is_short_record(&self) -> bool {
        self.0 & 0b0001_0000 > 0
    }

    pub fn chunk(&self) -> bool {
        self.0 & 0b0010_0000 > 0
    }

    pub fn message_end(&self) -> bool {
        self.0 & 0b0100_0000 > 0
    }

    pub fn message_begin(&self) -> bool {
        self.0 & 0b1000_0000 > 0
    }
}
