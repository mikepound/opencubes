//! Interaction with `.pcube` files

use std::{
    fs::File,
    io::{ErrorKind, Read, Seek, Write},
    iter::Peekable,
    path::Path,
};

mod raw_pcube;
pub use raw_pcube::RawPCube;

mod compression;
pub use compression::Compression;
use compression::{Reader, Writer};

use crate::iterator::{
    AllPolycubeIterator, AllUniquePolycubeIterator, PolycubeIterator, UniquePolycubeIterator,
};

const MAGIC: [u8; 4] = [0xCB, 0xEC, 0xCB, 0xEC];

/// A pcube file.
///
/// Use this file as an iterator to get all of the [`RawPCube`]s it contains.
pub struct PCubeFile<T = File>
where
    T: Read,
{
    input: Reader<T>,
    len: Option<usize>,
    cubes_read: usize,
    cubes_are_canonical: bool,
}

impl<T> Iterator for PCubeFile<T>
where
    T: Read,
{
    type Item = std::io::Result<RawPCube>;

    fn size_hint(&self) -> (usize, Option<usize>) {
        if let Some(len) = self.len {
            (len, Some(len))
        } else {
            (0, None)
        }
    }

    fn next(&mut self) -> Option<Self::Item> {
        self.next()
    }
}

impl<T> PCubeFile<T>
where
    T: Read,
{
    /// The compression used by this pcube file.
    pub fn compression(&self) -> Compression {
        match self.input {
            Reader::Uncompressed(_) => Compression::None,
            Reader::Gzip(_) => Compression::Gzip,
        }
    }

    /// The amount of polycubes in this file, if known.
    pub fn len(&self) -> Option<usize> {
        self.len
    }

    /// `true` if the file indicates that the cubes are
    /// in canonical form.
    pub fn canonical(&self) -> bool {
        self.cubes_are_canonical
    }

    /// Try to create a new [`PCubeFile`] from the provided byte source.
    pub fn new(mut input: T) -> std::io::Result<Self> {
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
            Ok(c) => Reader::new(c, input),
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
        })
    }

    pub fn next(&mut self) -> Option<std::io::Result<RawPCube>> {
        let next_cube = RawPCube::unpack(&mut self.input);

        match (next_cube, self.len) {
            (Ok(c), _) => {
                self.cubes_read += 1;
                Some(Ok(c))
            }
            (Err(_), None) => return None,
            (Err(e), Some(expected)) => {
                if expected == self.cubes_read {
                    return None;
                } else {
                    let msg = format!(
                        "Expected {expected} cubes, but failed to read after {} cubes. Error: {e}",
                        self.cubes_read
                    );
                    return Some(Err(std::io::Error::new(ErrorKind::InvalidData, msg)));
                }
            }
        }
    }

    pub fn into_iter(self) -> impl PolycubeIterator {
        IgnoreErrorIter::new(self)
    }

    /// This is by no means guaranteed, but makes life a bit easier
    pub fn assume_all_unique(self) -> AllUnique<T> {
        AllUnique::new(self)
    }
}

impl PCubeFile {
    /// Try to create a new [`PCubeFile`] from the given path.
    pub fn new_file(p: impl AsRef<Path>) -> std::io::Result<Self> {
        let file = std::fs::File::open(p.as_ref())?;
        Self::new(file)
    }

    /// Write implementation
    fn write_impl<I, W>(
        write_magic: bool,
        mut cubes: I,
        is_canonical: bool,
        compression: Compression,
        mut write: W,
    ) -> std::io::Result<()>
    where
        I: Iterator<Item = RawPCube>,
        W: Write,
    {
        if write_magic {
            write.write_all(&MAGIC)?;
        }

        let compression_val = compression.into();
        let orientation_val = if is_canonical { 1 } else { 0 };

        write.write_all(&[orientation_val, compression_val])?;

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

            write.write_all(&[next_byte])?;
        }

        let mut writer = Writer::new(compression, write);

        if let Some(e) = cubes.find_map(|v| v.pack(&mut writer).err()) {
            return Err(e);
        }

        Ok(())
    }

    /// Write the [`RawPCube`]s produced by `I` into `W`.
    ///
    /// `is_canonical` should only be set to `true` if all cubes in `I`
    /// are in canonical form.
    pub fn write<I, W>(
        is_canonical: bool,
        compression: Compression,
        cubes: I,
        write: W,
    ) -> std::io::Result<()>
    where
        I: Iterator<Item = RawPCube>,
        W: Write,
    {
        Self::write_impl(true, cubes, is_canonical, compression, write)
    }

    /// Write the [`RawPCube`]s produced by `I` to the file at `path`.
    ///
    /// This will create a new file, or _will_ overwrite the contents of the file at `path`.
    /// It will not create the parent directories of `path`.
    ///
    /// `is_canonical` should only be set to `true` if all cubes in `I`
    /// are in canonical form.
    ///
    /// The difference between [`PCubeFile::write_file`] and [`PCubeFile::write`] is
    /// that the former writes the magic bytes as the final step, while the latter
    /// does so immediately.
    pub fn write_file<I>(
        is_canonical: bool,
        compression: Compression,
        cubes: I,
        path: impl AsRef<Path>,
    ) -> std::io::Result<()>
    where
        I: Iterator<Item = RawPCube>,
    {
        let mut file = std::fs::File::create(path.as_ref())?;

        file.set_len(0)?;
        file.seek(std::io::SeekFrom::Start(0))?;
        file.write_all(&[0, 0, 0, 0])?;

        Self::write_impl(false, cubes, is_canonical, compression, &mut file)?;

        // Write magic last
        file.seek(std::io::SeekFrom::Start(0))?;
        file.write_all(&MAGIC)?;

        Ok(())
    }
}

struct IgnoreErrorIter<T>
where
    T: Read,
{
    inner: PCubeFile<T>,
}

impl<T> IgnoreErrorIter<T>
where
    T: Read,
{
    pub fn new(inner: PCubeFile<T>) -> Self {
        Self { inner }
    }
}

impl<T> Iterator for IgnoreErrorIter<T>
where
    T: Read,
{
    type Item = RawPCube;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|v| v.ok()).flatten()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> PolycubeIterator for IgnoreErrorIter<T>
where
    T: Read,
{
    fn is_canonical(&self) -> bool {
        self.inner.canonical()
    }

    fn n_hint(&self) -> Option<usize> {
        None
    }
}

pub struct AllUnique<T = File>
where
    T: Read,
{
    n: usize,
    canonical: bool,
    inner: Peekable<IgnoreErrorIter<T>>,
}

impl<T> AllUnique<T>
where
    T: Read,
{
    pub fn new(inner: PCubeFile<T>) -> Self {
        let canonical = inner.canonical();
        let mut peekable = IgnoreErrorIter::new(inner).peekable();

        let n = if let Some(peek) = peekable.peek() {
            let mut n = 0;
            let (x, y, z) = peek.dims();
            for x in 0..x {
                for y in 0..y {
                    for z in 0..z {
                        if peek.get(x, y, z) {
                            n += 1;
                        }
                    }
                }
            }

            n
        } else {
            0
        };

        Self {
            n,
            canonical,
            inner: peekable,
        }
    }
}

impl<T> Iterator for AllUnique<T>
where
    T: Read,
{
    type Item = RawPCube;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> PolycubeIterator for AllUnique<T>
where
    T: Read,
{
    fn is_canonical(&self) -> bool {
        self.canonical
    }

    fn n_hint(&self) -> Option<usize> {
        Some(self.n())
    }
}

impl<T> UniquePolycubeIterator for AllUnique<T> where T: Read {}

impl<T> AllPolycubeIterator for AllUnique<T>
where
    T: Read,
{
    fn n(&self) -> usize {
        self.n
    }
}

impl<T> AllUniquePolycubeIterator for AllUnique<T> where T: Read {}
