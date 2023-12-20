use crate::{Message, Oneof, OptionalMessage, RepeatedMessage, WireType};

/// Error returned by [`ByteReader`].
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct ReadError;

/// Reader for protobuf messages.
pub struct ByteReader<'a> {
    data: &'a [u8],
}

impl<'a> ByteReader<'a> {
    /// Create a new [`ByteReader`] that reads from `data`.
    pub fn new(data: &'a [u8]) -> Self {
        Self { data }
    }

    /// Get a reference to the remaining bytes.
    pub fn inner(&self) -> &[u8] {
        self.data
    }

    /// Check if the reader is at the end of the buffer.
    pub fn eof(&self) -> bool {
        self.data.is_empty()
    }

    /// Read `N` bytes from the buffer.
    pub fn read<const N: usize>(&mut self) -> Result<[u8; N], ReadError> {
        let n = self.data.get(0..N).ok_or(ReadError)?;
        self.data = &self.data[N..];
        Ok(n.try_into().unwrap())
    }

    /// Read a single byte from the buffer.
    pub fn read_u8(&mut self) -> Result<u8, ReadError> {
        Ok(u8::from_le_bytes(self.read()?))
    }

    /// Read a u16 from the buffer.
    pub fn read_u16(&mut self) -> Result<u16, ReadError> {
        Ok(u16::from_le_bytes(self.read()?))
    }

    /// Read a u32 from the buffer.
    pub fn read_u32(&mut self) -> Result<u32, ReadError> {
        Ok(u32::from_le_bytes(self.read()?))
    }

    /// Read a u64 from the buffer.
    pub fn read_u64(&mut self) -> Result<u64, ReadError> {
        Ok(u64::from_le_bytes(self.read()?))
    }

    /// Read a slice of length `len` from the buffer.
    pub fn read_slice(&mut self, len: usize) -> Result<&'a [u8], ReadError> {
        let res = self.data.get(0..len).ok_or(ReadError)?;
        self.data = &self.data[len..];
        Ok(res)
    }

    /// Read the remaining bytes from the buffer.
    pub fn read_to_end(&mut self) -> Result<&'a [u8], ReadError> {
        let res = self.data;
        self.data = &[];
        Ok(res)
    }

    /// Read a variable length slice from the buffer.
    pub fn read_varslice(&mut self) -> Result<&'a [u8], ReadError> {
        let len = self.read_varuint32()? as usize;
        self.read_slice(len)
    }

    /// Read varint-encoded bytes from the buffer.
    pub fn read_varuint_bytes(&mut self) -> Result<&'a [u8], ReadError> {
        for i in 0.. {
            if i >= self.data.len() {
                return Err(ReadError);
            }
            if self.data[i] & 0x80 == 0 {
                let res = &self.data[..i + 1];
                self.data = &self.data[i + 1..];
                return Ok(res);
            }
        }
        unreachable!()
    }

    /// Read varint-encoded u32 from the buffer.
    pub fn read_varuint32(&mut self) -> Result<u32, ReadError> {
        let mut res = 0;
        let mut shift = 0;
        loop {
            let x = self.read_u8()?;

            // avoid shift overflow if the varuint is more than 32bit.
            // this happpens in practice: negative int32's are encoded as 64bit two's complement
            // (in nanopb at least, I haven't checked other impls.)
            if shift < 32 {
                res |= (x as u32 & 0x7F) << shift;
            }

            if x & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        Ok(res)
    }

    /// Read a varint-encoded i32 from the buffer.
    pub fn read_varint32(&mut self) -> Result<i32, ReadError> {
        let u = self.read_varuint32()?;

        // zigzag encoding
        Ok(((u >> 1) as i32) ^ -((u & 1) as i32))
    }

    /// Read a varint-encoded u64 from the buffer.
    pub fn read_varuint64(&mut self) -> Result<u64, ReadError> {
        let mut res = 0;
        let mut shift = 0;
        loop {
            let x = self.read_u8()?;

            // avoid shift overflow if the varuint is more than 64bit.
            if shift < 64 {
                res |= (x as u64 & 0x7F) << shift;
            }

            if x & 0x80 == 0 {
                break;
            }
            shift += 7;
        }
        Ok(res)
    }

    /// Read a varint-encoded i64 from the buffer.
    pub fn read_varint64(&mut self) -> Result<i64, ReadError> {
        let u = self.read_varuint64()?;

        // zigzag encoding
        Ok(((u >> 1) as i64) ^ -((u & 1) as i64))
    }

    /// Return an iterator over the fields in the buffer.
    pub fn read_fields(&mut self) -> FieldIter<'_, 'a> {
        FieldIter { r: self }
    }
}

/// Iterator over the fields in a buffer.
pub struct FieldIter<'a, 'b> {
    r: &'a mut ByteReader<'b>,
}

impl<'a, 'b> Iterator for FieldIter<'a, 'b> {
    type Item = Result<FieldReader<'a>, ReadError>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.r.eof() {
            return None;
        }

        // Read header
        let header = match self.r.read_varuint32() {
            Ok(x) => x,
            Err(e) => return Some(Err(e)),
        };
        let tag = header >> 3;
        let wire_type = match header & 0b111 {
            0 => WireType::Varint,
            2 => WireType::LengthDelimited,
            _ => return Some(Err(ReadError)),
        };

        let data = match wire_type {
            WireType::Varint => match self.r.read_varuint_bytes() {
                Ok(x) => x,
                Err(e) => return Some(Err(e)),
            },
            WireType::LengthDelimited => {
                let len = match self.r.read_varuint32() {
                    Ok(x) => x as usize,
                    Err(e) => return Some(Err(e)),
                };

                match self.r.read_slice(len) {
                    Ok(x) => x,
                    Err(e) => return Some(Err(e)),
                }
            }
        };
        Some(Ok(FieldReader { tag, data, wire_type }))
    }
}

/// Reader for fields in a protobuf message.
pub struct FieldReader<'a> {
    tag: u32,
    data: &'a [u8],
    wire_type: WireType,
}

impl<'a> FieldReader<'a> {
    /// Get the tag of the field.
    pub fn tag(&self) -> u32 {
        self.tag
    }

    /// Read into a message of type `M`.
    pub fn read<M: Message>(self, msg: &mut M) -> Result<(), ReadError> {
        if self.wire_type != M::WIRE_TYPE {
            return Err(ReadError);
        }

        msg.read_raw(&mut ByteReader::new(self.data))
    }

    /// Read a repeated field into a message of type `M`.
    pub fn read_repeated<M: RepeatedMessage>(self, msg: &mut M) -> Result<(), ReadError> {
        if self.wire_type != M::Message::WIRE_TYPE {
            return Err(ReadError);
        }

        let mut m = M::Message::default();
        self.read(&mut m)?;
        msg.append(m)?;
        Ok(())
    }

    /// Read an optional field into a message of type `M`.
    pub fn read_optional<M: OptionalMessage>(self, msg: &mut M) -> Result<(), ReadError> {
        if self.wire_type != M::Message::WIRE_TYPE {
            return Err(ReadError);
        }

        let mut m = M::Message::default();
        self.read(&mut m)?;
        msg.set(m)?;
        Ok(())
    }

    /// Read a oneof field into a message of type `M`.
    pub fn read_oneof<M: Oneof>(self, msg: &mut M) -> Result<(), ReadError> {
        msg.read_raw(self)
    }

    /// Read a oneof variant into a message of type `M`.
    pub fn read_oneof_variant<M: Message + Default>(self) -> Result<M, ReadError> {
        if self.wire_type != M::WIRE_TYPE {
            return Err(ReadError);
        }

        let mut msg: M = Default::default();
        msg.read_raw(&mut ByteReader::new(self.data))?;
        Ok(msg)
    }
}
