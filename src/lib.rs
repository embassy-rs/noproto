#![doc = include_str!("../README.md")]
#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

mod impls;
mod read;
mod write;

pub use read::ReadError;
use read::{ByteReader, FieldReader};
use write::ByteWriter;
pub use write::WriteError;

pub mod encoding {
    //! Encoding and decoding of primitive types.
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

/// Wire type of a field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum WireType {
    /// Varint.
    Varint = 0,
    //SixtyFourBit = 1,
    /// Length-delimited.
    LengthDelimited = 2,
    //StartGroup = 3,
    //EndGroup = 4,
    //ThirtyTwoBit = 5,
}

/// A protobuf message.
pub trait Message {
    /// The wire type of the message.
    const WIRE_TYPE: WireType;
    /// Serialize the message.
    fn write_raw(&self, w: &mut ByteWriter) -> Result<(), WriteError>;
    /// Deserialize the message.
    fn read_raw(&mut self, r: &mut ByteReader) -> Result<(), ReadError>;
}

/// An optional protobuf message.
pub trait OptionalMessage {
    /// The message type.
    type Message: Message + Default;

    /// Get the message, if it exists.
    fn get(&self) -> Option<&Self::Message>;
    /// Set the message.
    fn set(&mut self, m: Self::Message) -> Result<(), ReadError>;
}

/// A repeated protobuf message.
pub trait RepeatedMessage {
    /// The message type.
    type Message: Message + Default;
    /// An iterator over the messages.
    type Iter<'a>: Iterator<Item = &'a Self::Message>
    where
        Self: 'a;

    /// Get an iterator over the messages.
    fn iter(&self) -> Result<Self::Iter<'_>, WriteError>;
    /// Append a message.
    fn append(&mut self, m: Self::Message) -> Result<(), ReadError>;
}

/// A oneof protobuf message.
pub trait Oneof: Sized {
    /// Serialize the message.
    fn write_raw(&self, w: &mut ByteWriter) -> Result<(), WriteError>;
    /// Deserialize the message.
    fn read_raw(&mut self, r: FieldReader) -> Result<(), ReadError>;
    /// Deserialize a oneof variant.
    fn read_raw_option(this: &mut Option<Self>, r: FieldReader) -> Result<(), ReadError>;
}

/// Serialize a protobuf message to a buffer.
pub fn write<M: Message>(msg: &M, buf: &mut [u8]) -> Result<usize, WriteError> {
    let mut w = ByteWriter::new(buf);
    msg.write_raw(&mut w)?;
    Ok(w.pos())
}

/// Deserialize a protobuf message from a buffer.
pub fn read<M: Message + Default>(buf: &[u8]) -> Result<M, ReadError> {
    let mut msg = M::default();
    let mut r = ByteReader::new(buf);
    msg.read_raw(&mut r)?;
    Ok(msg)
}
