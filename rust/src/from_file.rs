use std::{
    fs::File,
    io::{BufReader, ErrorKind, Read, Seek, Write},
    path::Path,
};

use flate2::{read::GzDecoder, write::GzEncoder};

use crate::PolyCube;

const MAGIC: [u8; 4] = [0xCB, 0xEC, 0xCB, 0xEC];

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

enum Reader<T>
where
    T: Read,
{
    Uncompressed(BufReader<T>),
    Gzip(GzDecoder<T>),
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

enum Writer<T>
where
    T: Write,
{
    Uncompressed(T),
    Gzip(GzEncoder<T>),
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
        todo!()
    }
}

pub struct PolyCubeFile<T = File>
where
    T: Read,
{
    pub should_canonicalize: bool,
    had_error: bool,
    input: Reader<T>,
    len: Option<usize>,
    cubes_read: usize,
    cubes_are_canonical: bool,
}

impl<T> Iterator for PolyCubeFile<T>
where
    T: Read,
{
    type Item = std::io::Result<PolyCube>;

    fn size_hint(&self) -> (usize, Option<usize>) {
        if let Some(len) = self.len {
            (0, Some(len))
        } else {
            (0, None)
        }
    }

    fn next(&mut self) -> Option<Self::Item> {
        if self.had_error {
            return None;
        }

        let next_cube = PolyCube::unpack(&mut self.input);

        let mut next_cube = match (next_cube, self.len) {
            (Err(_), None) => return None,
            (Err(e), Some(expected)) => {
                if expected == self.cubes_read {
                    return None;
                } else {
                    self.had_error = true;
                    let msg = format!(
                        "Expected {expected} cubes, but failed to read after {} cubes. Error: {e}",
                        self.cubes_read
                    );
                    return Some(Err(std::io::Error::new(ErrorKind::InvalidData, msg)));
                }
            }
            (Ok(c), _) => c,
        };

        if !self.cubes_are_canonical && self.should_canonicalize {
            next_cube = next_cube
                .all_rotations()
                .max_by(PolyCube::canonical_ordering)
                .unwrap();
        }

        self.cubes_read += 1;

        Some(Ok(next_cube))
    }
}

impl PolyCubeFile {
    pub fn new(p: impl AsRef<Path>) -> std::io::Result<Self> {
        let file = std::fs::File::open(p.as_ref())?;
        Self::new_from_read(file)
    }

    pub fn write<C, I>(
        mut cubes: I,
        is_canonical: bool,
        compression: Compression,
        mut file: File,
    ) -> std::io::Result<()>
    where
        I: Iterator<Item = C>,
        C: std::borrow::Borrow<PolyCube>,
    {
        file.set_len(0)?;

        file.write_all(&[0, 0, 0, 0])?;

        let compression_val = compression.into();
        let orientation_val = if is_canonical { 1 } else { 0 };

        file.write_all(&[orientation_val, compression_val])?;

        let mut cube_count = 0;
        let (_, max) = cubes.size_hint();

        if let Some(max) = max {
            cube_count = max;
        }

        let mut ran_once = false;
        while cube_count > 0 || !ran_once {
            ran_once = true;
            let mut next_byte = (cube_count as u8) & 0x7F;
            cube_count >>= 7;

            if cube_count > 0 {
                next_byte |= 0x80;
            }

            file.write_all(&[next_byte])?;
        }

        let mut writer = match compression {
            Compression::None => Writer::Uncompressed(&mut file),
            Compression::Gzip => {
                Writer::Gzip(GzEncoder::new(&mut file, flate2::Compression::default()))
            }
        };

        if let Some(e) = cubes.find_map(|v| v.borrow().pack(&mut writer).err()) {
            return Err(e);
        }

        drop(writer);

        // Finally, write the magic
        file.seek(std::io::SeekFrom::Start(0))?;
        file.write(&MAGIC)?;

        Ok(())
    }
}

impl<T> PolyCubeFile<T>
where
    T: Read,
{
    pub fn compression(&self) -> Compression {
        match self.input {
            Reader::Uncompressed(_) => Compression::None,
            Reader::Gzip(_) => Compression::Gzip,
        }
    }

    pub fn len(&self) -> Option<usize> {
        self.len
    }

    pub fn canonical(&self) -> bool {
        self.cubes_are_canonical
    }

    pub fn new_from_read(mut input: T) -> std::io::Result<Self> {
        let mut magic = [0u8; 4];
        input.read_exact(&mut magic)?;

        if magic != MAGIC {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "File magic was incorrect.",
            ));
        }

        let mut header = [0u8; 2];
        input.read_exact(&mut header)?;

        let [orientation, compression] = header;
        let canonicalized = orientation != 0;

        let mut cube_count: u64 = 0;
        let mut shift = 0;
        loop {
            let mut next_byte = [0u8; 1];
            input.read_exact(&mut next_byte)?;

            let [next_byte] = next_byte;

            cube_count |= ((next_byte & 0x7F) as u64) << shift;

            shift += 7;
            if shift > 64 {
                panic!("Cannot load possibly more than u64 cubes...");
            }

            if next_byte & 0x80 == 0 {
                break;
            }
        }

        let len = if cube_count == 0 {
            None
        } else {
            Some(cube_count as usize)
        };

        let input = match Compression::try_from(compression) {
            Ok(Compression::None) => Reader::Uncompressed(BufReader::new(input)),
            Ok(Compression::Gzip) => Reader::Gzip(GzDecoder::new(input)),
            Err(_) => {
                return Err(std::io::Error::new(
                    ErrorKind::InvalidData,
                    format!("Unsupported compression type {compression}"),
                ))
            }
        };

        Ok(Self {
            input,
            len,
            cubes_read: 0,
            cubes_are_canonical: canonicalized,
            should_canonicalize: true,
            had_error: false,
        })
    }
}
