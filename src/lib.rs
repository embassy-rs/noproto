#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]

mod impls;
mod read;
mod write;

pub use read::ReadError;
use read::{ByteReader, FieldReader};
use write::ByteWriter;
pub use write::WriteError;

pub mod encoding {
    pub use crate::read::*;
    pub use crate::write::*;
}

// Re-export #[derive(Message, Enumeration, Oneof)].
#[cfg(feature = "derive")]
#[allow(unused_imports)]
#[macro_use]
extern crate noproto_derive;
#[cfg(feature = "derive")]
#[doc(hidden)]
pub use noproto_derive::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum WireType {
    Varint = 0,
    //SixtyFourBit = 1,
    LengthDelimited = 2,
    //StartGroup = 3,
    //EndGroup = 4,
    //ThirtyTwoBit = 5,
}

pub trait Message {
    const WIRE_TYPE: WireType;
    fn write_raw(&self, w: &mut ByteWriter) -> Result<(), WriteError>;
    fn read_raw(&mut self, r: &mut ByteReader) -> Result<(), ReadError>;
}

pub trait OptionalMessage {
    type Message: Message + Default;

    fn get(&self) -> Option<&Self::Message>;
    fn set(&mut self, m: Self::Message) -> Result<(), ReadError>;
}

pub trait RepeatedMessage {
    type Message: Message + Default;
    type Iter<'a>: Iterator<Item = &'a Self::Message>
    where
        Self: 'a;

    fn iter(&self) -> Result<Self::Iter<'_>, WriteError>;
    fn append(&mut self, m: Self::Message) -> Result<(), ReadError>;
}

pub trait Oneof: Sized {
    fn write_raw(&self, w: &mut ByteWriter) -> Result<(), WriteError>;
    fn read_raw(&mut self, r: FieldReader) -> Result<(), ReadError>;
    fn read_raw_option(this: &mut Option<Self>, r: FieldReader) -> Result<(), ReadError>;
}

pub fn write<M: Message>(msg: &M, buf: &mut [u8]) -> Result<usize, WriteError> {
    let mut w = ByteWriter::new(buf);
    msg.write_raw(&mut w)?;
    Ok(w.pos())
}

pub fn read<M: Message + Default>(buf: &[u8]) -> Result<M, ReadError> {
    let mut msg = M::default();
    let mut r = ByteReader::new(buf);
    msg.read_raw(&mut r)?;
    Ok(msg)
}
