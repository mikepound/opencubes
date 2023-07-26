use std::io::{BufReader, Read, Write};

use flate2::{read::GzDecoder, write::GzEncoder};

/// Compression types supported for `.pcube` files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Compression {
    None,
    Gzip,
}

impl From<Compression> for u8 {
    fn from(value: Compression) -> Self {
        match value {
            Compression::None => 0,
            Compression::Gzip => 1,
        }
    }
}

impl TryFrom<u8> for Compression {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        let compression = match value {
            0 => Self::None,
            1 => Self::Gzip,
            _ => return Err(()),
        };
        Ok(compression)
    }
}

pub enum Reader<T>
where
    T: Read,
{
    Uncompressed(BufReader<T>),
    Gzip(GzDecoder<T>),
}

impl<T> Reader<T>
where
    T: Read,
{
    pub fn new(compression: Compression, reader: T) -> Self {
        match compression {
            Compression::None => Self::Uncompressed(BufReader::new(reader)),
            Compression::Gzip => Self::Gzip(GzDecoder::new(reader)),
        }
    }
}

impl<T> Read for Reader<T>
where
    T: Read,
{
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Reader::Uncompressed(t) => t.read(buf),
            Reader::Gzip(t) => t.read(buf),
        }
    }

    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        match self {
            Reader::Uncompressed(t) => t.read_exact(buf),
            Reader::Gzip(t) => t.read_exact(buf),
        }
    }
}

pub enum Writer<T>
where
    T: Write,
{
    Uncompressed(T),
    Gzip(GzEncoder<T>),
}

impl<T> Writer<T>
where
    T: Write,
{
    pub fn new(compression: Compression, writer: T) -> Self {
        match compression {
            Compression::None => Self::Uncompressed(writer),
            Compression::Gzip => Self::Gzip(GzEncoder::new(writer, flate2::Compression::default())),
        }
    }
}

impl<T> Write for Writer<T>
where
    T: Write,
{
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Writer::Uncompressed(t) => t.write(buf),
            Writer::Gzip(t) => t.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Writer::Uncompressed(t) => t.flush(),
            Writer::Gzip(t) => t.flush(),
        }
    }
}
