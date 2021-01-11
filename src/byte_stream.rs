use std::io::{Seek, Result, SeekFrom};
use omnom::{ReadBytes, ReadExt};

/// Used to decide whether read_be or read_le is called
#[derive(Clone, Copy)]
pub enum ByteOrder {
  LittleEndian,
  BigEndian,
}

/// Structure that holds our read stream and also the mode (byte order) of reading
pub struct ByteStream<R: ReadExt> {
  order: ByteOrder,
  pub reader: R,
}

impl <R: ReadExt> ByteStream<R> {

  /// Construct a new ByteStream with a specific byte order (LE or BE)
  pub fn new(reader: R, order: ByteOrder) -> Self {
    ByteStream {
      order,
      reader
    }
  }

  /// Change the order
  pub fn set_order(&mut self, order: ByteOrder) {
    self.order = order;
  }

  /// Read from the stream with the specified order, overriding the ByteStream order
  pub fn read_with_order<B: ReadBytes>(&mut self, order: ByteOrder) -> Result<B> {
    match order {
      ByteOrder::LittleEndian => self.reader.read_le(),
      ByteOrder::BigEndian => self.reader.read_be()
    }
  }

  /// Read function matches on the current read mode and reads using it (either LittleEndian or BigEndian).
  /// Uses methods from omnom library ReadExt to take from the reader
  pub fn read<B: ReadBytes>(&mut self) -> Result<B> {
    self.read_with_order(self.order)
  }

  pub fn read_bytes(&mut self, buf: &mut [u8]) -> Result<()> {
    self.reader.read_exact(buf)
  }
}

impl <R: Seek + ReadExt> ByteStream<R> {
  pub fn seek(&mut self, pos: SeekFrom) -> Result<u64> {
    self.reader.seek(pos)
  }
}
