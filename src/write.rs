use crate::{Message, Oneof, OptionalMessage, RepeatedMessage, WireType};

/// Error returned by [`ByteWriter`].
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct WriteError;

/// Writer for protobuf messages.
pub struct ByteWriter<'a> {
    buf: &'a mut [u8],
    pos: usize,
}

impl<'a> ByteWriter<'a> {
    /// Create a new [`ByteWriter`] that writes to `buf`.
    pub fn new(buf: &'a mut [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    /// Get the bytes written so far.
    pub fn bytes(&self) -> &[u8] {
        &self.buf[..self.pos]
    }

    /// Write `bytes` to the buffer.
    pub fn write(&mut self, bytes: &[u8]) -> Result<(), WriteError> {
        if self.buf.len() - self.pos < bytes.len() {
            return Err(WriteError);
        }
        self.buf[self.pos..][..bytes.len()].copy_from_slice(bytes);
        self.pos += bytes.len();
        Ok(())
    }

    /// Write a single byte to the buffer.
    pub fn write_u8(&mut self, val: u8) -> Result<(), WriteError> {
        self.write(&val.to_le_bytes())
    }

    /// Write u16 to the buffer.
    pub fn write_u16(&mut self, val: u16) -> Result<(), WriteError> {
        self.write(&val.to_le_bytes())
    }

    /// Write u32 to the buffer.
    pub fn write_u32(&mut self, val: u32) -> Result<(), WriteError> {
        self.write(&val.to_le_bytes())
    }

    /// Write u64 to the buffer.
    pub fn write_u64(&mut self, val: u64) -> Result<(), WriteError> {
        self.write(&val.to_le_bytes())
    }

    /// Write varint-encoded u32 to the buffer.
    pub fn write_varuint32(&mut self, mut val: u32) -> Result<(), WriteError> {
        loop {
            let mut part = val & 0x7F;
            let rest = val >> 7;
            if rest != 0 {
                part |= 0x80
            }

            self.write_u8(part as u8)?;

            if rest == 0 {
                return Ok(());
            }
            val = rest
        }
    }

    /// Write varint-encoded u64 to the buffer.
    pub fn write_varuint64(&mut self, mut val: u64) -> Result<(), WriteError> {
        loop {
            let mut part = val & 0x7F;
            let rest = val >> 7;
            if rest != 0 {
                part |= 0x80
            }

            self.write_u8(part as u8)?;

            if rest == 0 {
                return Ok(());
            }
            val = rest
        }
    }

    /// Write varint-encoded i32 to the buffer.
    pub fn write_varint32(&mut self, val: i32) -> Result<(), WriteError> {
        self.write_varuint32(((val >> 31) ^ (val << 1)) as u32)
    }

    /// Write varint-encoded i64 to the buffer.
    pub fn write_varint64(&mut self, val: i64) -> Result<(), WriteError> {
        self.write_varuint64(((val >> 63) ^ (val << 1)) as u64)
    }

    /// Write length-delimited data to the buffer.
    pub fn write_length_delimited(
        &mut self,
        f: impl FnOnce(&mut ByteWriter) -> Result<(), WriteError>,
    ) -> Result<(), WriteError> {
        // Write the data
        let start = self.pos;
        f(self)?;
        let len = self.pos - start;

        // Encode length header
        let mut header = [0; 16];
        let mut header = ByteWriter::new(&mut header);
        header.write_varuint32(len.try_into().unwrap())?;
        let header = header.bytes();

        // Move the data to make space for the header.
        if self.buf.len() - self.pos < header.len() {
            return Err(WriteError);
        }
        self.buf.copy_within(start..self.pos, start + header.len());

        // Insert the header
        self.buf[start..][..header.len()].copy_from_slice(header);
        self.pos += header.len();

        Ok(())
    }

    /// Write a protobuf field to the buffer.
    pub fn write_field<M: Message>(&mut self, tag: u32, msg: &M) -> Result<(), WriteError> {
        self.write_varuint32((tag << 3) | (M::WIRE_TYPE as u32))?;

        match M::WIRE_TYPE {
            WireType::LengthDelimited => self.write_length_delimited(|w| msg.write_raw(w)),
            _ => msg.write_raw(self),
        }
    }

    /// Write a repeated protobuf field to the buffer.
    pub fn write_repeated<M: RepeatedMessage>(&mut self, tag: u32, msg: &M) -> Result<(), WriteError> {
        for i in msg.iter()? {
            self.write_field(tag, i)?;
        }
        Ok(())
    }

    /// Write an optional protobuf field to the buffer.
    pub fn write_optional<M: OptionalMessage>(&mut self, tag: u32, msg: &M) -> Result<(), WriteError> {
        if let Some(msg) = msg.get() {
            self.write_field(tag, msg)?;
        }
        Ok(())
    }

    /// Write a oneof protobuf field to the buffer.
    pub fn write_oneof<M: Oneof>(&mut self, msg: &M) -> Result<(), WriteError> {
        msg.write_raw(self)
    }

    pub(crate) fn pos(&self) -> usize {
        self.pos
    }
}
