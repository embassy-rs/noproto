use crate::read::ByteReader;
use crate::write::ByteWriter;
use crate::{Message, Oneof, OptionalMessage, ReadError, RepeatedMessage, WireType, WriteError};

impl Message for bool {
    const WIRE_TYPE: WireType = WireType::Varint;
    fn write_raw(&self, w: &mut ByteWriter) -> Result<(), WriteError> {
        w.write_varuint32(*self as _)
    }
    fn read_raw(&mut self, r: &mut ByteReader) -> Result<(), ReadError> {
        let val = r.read_varuint32()?;

        *self = match val {
            0 => false,
            1 => true,
            _ => return Err(ReadError),
        };
        Ok(())
    }
}

impl Message for u8 {
    const WIRE_TYPE: WireType = WireType::Varint;
    fn write_raw(&self, w: &mut ByteWriter) -> Result<(), WriteError> {
        w.write_varuint32(*self as _)
    }
    fn read_raw(&mut self, r: &mut ByteReader) -> Result<(), ReadError> {
        *self = r.read_varuint32()?.try_into().map_err(|_| ReadError)?;
        Ok(())
    }
}

impl Message for u16 {
    const WIRE_TYPE: WireType = WireType::Varint;
    fn write_raw(&self, w: &mut ByteWriter) -> Result<(), WriteError> {
        w.write_varuint32(*self as _)
    }
    fn read_raw(&mut self, r: &mut ByteReader) -> Result<(), ReadError> {
        *self = r.read_varuint32()?.try_into().map_err(|_| ReadError)?;
        Ok(())
    }
}

impl Message for u32 {
    const WIRE_TYPE: WireType = WireType::Varint;
    fn write_raw(&self, w: &mut ByteWriter) -> Result<(), WriteError> {
        w.write_varuint32(*self)
    }
    fn read_raw(&mut self, r: &mut ByteReader) -> Result<(), ReadError> {
        *self = r.read_varuint32()?.try_into().map_err(|_| ReadError)?;
        Ok(())
    }
}

impl Message for u64 {
    const WIRE_TYPE: WireType = WireType::Varint;
    fn write_raw(&self, w: &mut ByteWriter) -> Result<(), WriteError> {
        w.write_varuint64(*self)
    }
    fn read_raw(&mut self, r: &mut ByteReader) -> Result<(), ReadError> {
        *self = r.read_varuint64()?.try_into().map_err(|_| ReadError)?;
        Ok(())
    }
}

impl Message for i8 {
    const WIRE_TYPE: WireType = WireType::Varint;
    fn write_raw(&self, w: &mut ByteWriter) -> Result<(), WriteError> {
        w.write_varint32(*self as _)
    }
    fn read_raw(&mut self, r: &mut ByteReader) -> Result<(), ReadError> {
        *self = r.read_varint32()?.try_into().map_err(|_| ReadError)?;
        Ok(())
    }
}

impl Message for i16 {
    const WIRE_TYPE: WireType = WireType::Varint;
    fn write_raw(&self, w: &mut ByteWriter) -> Result<(), WriteError> {
        w.write_varint32(*self as _)
    }
    fn read_raw(&mut self, r: &mut ByteReader) -> Result<(), ReadError> {
        *self = r.read_varint32()?.try_into().map_err(|_| ReadError)?;
        Ok(())
    }
}

impl Message for i32 {
    const WIRE_TYPE: WireType = WireType::Varint;
    fn write_raw(&self, w: &mut ByteWriter) -> Result<(), WriteError> {
        w.write_varint32(*self)
    }
    fn read_raw(&mut self, r: &mut ByteReader) -> Result<(), ReadError> {
        *self = r.read_varint32()?.try_into().map_err(|_| ReadError)?;
        Ok(())
    }
}

impl Message for i64 {
    const WIRE_TYPE: WireType = WireType::Varint;
    fn write_raw(&self, w: &mut ByteWriter) -> Result<(), WriteError> {
        w.write_varint64(*self)
    }
    fn read_raw(&mut self, r: &mut ByteReader) -> Result<(), ReadError> {
        *self = r.read_varint64()?.try_into().map_err(|_| ReadError)?;
        Ok(())
    }
}

impl<const N: usize> Message for heapless::String<N> {
    const WIRE_TYPE: WireType = WireType::LengthDelimited;
    fn write_raw(&self, w: &mut ByteWriter) -> Result<(), WriteError> {
        w.write(self.as_bytes())
    }
    fn read_raw(&mut self, r: &mut ByteReader) -> Result<(), ReadError> {
        let data = r.read_to_end()?;
        let data = core::str::from_utf8(data).map_err(|_| ReadError)?;
        self.clear();
        self.push_str(data).map_err(|_| ReadError)?;
        Ok(())
    }
}

impl<const N: usize> Message for heapless::Vec<u8, N> {
    const WIRE_TYPE: WireType = WireType::LengthDelimited;
    fn write_raw(&self, w: &mut ByteWriter) -> Result<(), WriteError> {
        w.write(self)
    }
    fn read_raw(&mut self, r: &mut ByteReader) -> Result<(), ReadError> {
        let data = r.read_to_end()?;
        self.clear();
        self.extend_from_slice(data).map_err(|_| ReadError)?;
        Ok(())
    }
}

impl<M: Message + Default, const N: usize> RepeatedMessage for heapless::Vec<M, N> {
    type Message = M;

    type Iter<'a> = core::slice::Iter<'a, M> where Self: 'a ;

    fn iter(&self) -> Result<Self::Iter<'_>, WriteError> {
        Ok(self[..].iter())
    }

    fn append(&mut self, m: Self::Message) -> Result<(), ReadError> {
        self.push(m).map_err(|_| ReadError)
    }
}

impl<M: Message + Default> OptionalMessage for Option<M> {
    type Message = M;

    fn get(&self) -> Option<&Self::Message> {
        self.as_ref()
    }

    fn set(&mut self, m: Self::Message) -> Result<(), ReadError> {
        *self = Some(m);
        Ok(())
    }
}

impl<M: Oneof> Oneof for Option<M> {
    fn write_raw(&self, w: &mut ByteWriter) -> Result<(), WriteError> {
        if let Some(x) = self {
            x.write_raw(w)?;
        }
        Ok(())
    }

    fn read_raw(&mut self, r: crate::encoding::FieldReader) -> Result<(), ReadError> {
        M::read_raw_option(self, r)
    }

    fn read_raw_option(_this: &mut Option<Self>, _r: crate::encoding::FieldReader) -> Result<(), ReadError> {
        panic!("cannot nest options with oneof.")
    }
}
