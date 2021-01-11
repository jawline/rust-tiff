mod byte_stream;

use std::{result::Result, error::Error, fs::File, io::Read};
use std::io::{Cursor, Seek, SeekFrom};
use std::convert::TryInto;

use omnom::prelude::*;

use byte_stream::ByteOrder;
use byte_stream::ByteStream;

type BR<T> = Result<T, Box<dyn Error>>;

fn open_raw_file(path: &str) -> Result<ByteStream<File>, Box<dyn Error>> {
  let mut file = File::open(path)?;
  let byte_1: u8 = file.read_le()?;
  let byte_2: u8 = file.read_le()?;

  let intel_order = byte_1 == 0x49 && byte_2 == 0x49;
  let motorola_order = byte_1 == 0x4D && byte_2 == 0x4D;

  if intel_order {
    Ok(ByteStream::new(file, ByteOrder::LittleEndian))
  } else if motorola_order {
    Ok(ByteStream::new(file, ByteOrder::BigEndian))
  } else {
    Err("not a valid file".into())
  }
}

#[derive(Debug)]
struct TiffHeader {
  pub magic: u16,
  pub ifd_start: u32,
}

impl TiffHeader {
  pub fn parse<T: Read>(stream: &mut ByteStream<T>) -> BR<TiffHeader> {
    let hdr = TiffHeader {
      magic: stream.read()?,
      ifd_start: stream.read()?,
    };

    if hdr.magic != 42 {
      return Err("not a TIFF file".into());
    }

    Ok(hdr)
  }
}

#[derive(Debug)]
struct IFE {
  pub tag: u16,
  pub entry_type: u16,
  pub count: u32,
  pub offset: u32,
}

impl IFE {
  pub fn parse<T: Read>(stream: &mut ByteStream<T>) -> BR<IFE> {

    let hdr = IFE {
      tag: stream.read()?,
      entry_type: stream.read()?,
      count: stream.read()?,
      offset: stream.read()?,
    };

    Ok(hdr)
  }

  pub fn seek_to<T: Read + Seek>(&self, stream: &mut ByteStream<T>) -> BR<()> {
    stream.seek(SeekFrom::Start(self.offset.into()))?;
    Ok(())
  }

  /// Read this entries contents as bytes
  pub fn read_as_bytes<T: Read + Seek>(&self, stream: &mut ByteStream<T>) -> BR<Vec<u8>> {

    self.seek_to(stream)?;

    let mut buffer = vec![0; self.count.try_into()?];
    stream.read_bytes(&mut buffer)?;

    Ok(buffer)
  }

  /// Parse a short (one unsigned 16 bit value)
  pub fn to_short<T: Read + Seek>(&self, stream: &mut ByteStream<T>) -> BR<u16> {

    if self.entry_type != 3 {
      return Err("not short".into());
    }

    self.seek_to(stream)?;
    Ok(stream.read()?)
  }

  /// Parse a long (one unsigned 32 bit value)
  pub fn to_long<T: Read + Seek>(&self, stream: &mut ByteStream<T>) -> BR<u32> {

    if self.entry_type != 4 {
      return Err("not long".into());
    }

    self.seek_to(stream)?;
    Ok(stream.read()?)
  }

  /// Parse a rational (two long values in the format numerator/denomenator)
  pub fn to_rational<T: Read + Seek>(&self, stream: &mut ByteStream<T>) -> BR<f64> {

    if self.entry_type != 5 {
      return Err("not rational".into());
    }

    self.seek_to(stream)?;

    let numerator: u32 = stream.read()?;
    let denominator: u32 = stream.read()?;

    let rational = numerator as f64 / denominator as f64;

    Ok(rational)
  }

  /// If this is a text entry convert it to UTF8 (Should be ASCII encoded). Last char should be a null terminator
  pub fn to_ascii<T: Read + Seek>(&self, stream: &mut ByteStream<T>) -> BR<String> {

    if self.entry_type != 2 {
      return Err("not ascii".into());
    }

    let buffer = self.read_as_bytes(stream)?;

    Ok(std::str::from_utf8(&buffer)?.to_string())
  }
}

#[derive(Debug)]
struct IFD {
  entries: Vec<IFE>,
}

impl IFD {
  pub fn parse<T: Read + Seek>(stream: &mut ByteStream<T>) -> BR<Vec<IFD>> {

    let count: u16 = stream.read()?;

    let mut result = Vec::new();

    for i in 0..count {
      result.push(IFE::parse(stream)?);
    }

    let read_ifd = IFD {
      entries: result
    };

    let next: u32 = stream.read()?;

    if next == 0 {
      Ok(vec!(read_ifd))
    } else {
      // If there's still stuff to read then seek to it and parse
      stream.seek(SeekFrom::Start(next.into()))?;
      let mut remaining = IFD::parse(stream)?;
      remaining.push(read_ifd);
      Ok(remaining)
    }
  }
}

fn main() -> Result<(), Box<dyn Error>> {
  let mut stream = open_raw_file("./test.arw")?;

  let header = TiffHeader::parse(&mut stream)?;

  println!("{:?}", header);

  // Now read the directories
  stream.seek(SeekFrom::Start(header.ifd_start.into()))?;
  let ifds = IFD::parse(&mut stream)?;

  for dir in ifds {
    println!("--- NEW IFD ----");
    for entry in dir.entries {
      println!("{:?}", entry);
      match entry.entry_type {
        2 => {
          println!("{}", entry.to_ascii(&mut stream)?);
        },
        3 => {
          println!("{}", entry.to_short(&mut stream)?);
        },
        4 => {
          println!("{}", entry.to_long(&mut stream)?);
        },
        5 => {
          println!("{}", entry.to_rational(&mut stream)?);
        },
        _ => { println!("Unknown - Skip"); },
      }
    }
  }

  Ok(())
}
