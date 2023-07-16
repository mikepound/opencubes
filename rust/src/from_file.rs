use std::{
    fs::File,
    io::{ErrorKind, Read, Write},
    path::Path,
    sync::{atomic::AtomicUsize, Arc},
};

use crate::PolyCube;

const MAGIC: [u8; 4] = [0xCB, 0xEC, 0xCB, 0xEC];

pub struct PolyCubeFileReader {
    file: File,
    len: Option<usize>,
    cubes_read: usize,
    pub should_canonicalize: bool,
    cubes_are_canonical: bool,
    alloc_count: Arc<AtomicUsize>,
}

impl Iterator for PolyCubeFileReader {
    type Item = std::io::Result<PolyCube>;

    fn size_hint(&self) -> (usize, Option<usize>) {
        if let Some(len) = self.len {
            (len, Some(len))
        } else {
            (0, None)
        }
    }

    fn next(&mut self) -> Option<Self::Item> {
        let next_cube = PolyCube::unpack_with(self.alloc_count.clone(), &mut self.file);

        let mut next_cube = match (next_cube, self.len) {
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

impl PolyCubeFileReader {
    pub fn len(&self) -> Option<usize> {
        self.len
    }

    pub fn canonical(&self) -> bool {
        self.cubes_are_canonical
    }

    pub fn new(p: impl AsRef<Path>) -> std::io::Result<Self> {
        let file = std::fs::File::open(p.as_ref())?;
        Self::new_from_file(file)
    }

    pub fn new_from_file(mut file: File) -> std::io::Result<Self> {
        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;

        if magic != MAGIC {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "File magic was incorrect.",
            ));
        }

        let mut header = [0u8; 2];
        file.read_exact(&mut header)?;

        let [orientation, compression] = header;
        let canonicalized = orientation != 0;

        if compression != 0 {
            return Err(std::io::Error::new(
                ErrorKind::Other,
                "Compression is not supported",
            ));
        }

        let mut cube_count: u64 = 0;
        let mut shift = 0;
        loop {
            let mut next_byte = [0u8; 1];
            file.read_exact(&mut next_byte)?;

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

        Ok(Self {
            file,
            len,
            cubes_read: 0,
            cubes_are_canonical: canonicalized,
            alloc_count: Arc::new(AtomicUsize::new(0)),
            should_canonicalize: true,
        })
    }

    pub fn to_file<C, I>(mut cubes: I, canonical: bool, mut file: File) -> std::io::Result<()>
    where
        I: Iterator<Item = C>,
        C: std::borrow::Borrow<PolyCube>,
    {
        file.set_len(0)?;

        file.write_all(&MAGIC)?;

        let compression = 0;
        let orientation = if canonical { 1 } else { 0 };

        file.write_all(&[orientation, compression])?;

        let mut cube_count = 0;
        let (min, max) = cubes.size_hint();

        if Some(min) == max {
            cube_count = min;
        }

        while cube_count > 0 {
            let mut next_byte = (cube_count as u8) & 0x7F;
            cube_count >>= 7;

            if cube_count > 0 {
                next_byte |= 0x80;
            }

            file.write_all(&[next_byte])?;
        }

        if let Some(e) = cubes.find_map(|v| v.borrow().pack(&mut file).err()) {
            return Err(e);
        }

        Ok(())
    }
}
