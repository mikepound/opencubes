use std::{
    collections::HashSet,
    io::{ErrorKind, Read},
    path::Path,
    sync::{atomic::AtomicUsize, Arc},
};

use crate::PolyCube;

pub struct PolyCubeFromFileReader;

impl PolyCubeFromFileReader {
    pub fn from_file(path: impl AsRef<Path>) -> std::io::Result<Vec<PolyCube>> {
        let path = path.as_ref();

        let mut file = std::fs::File::open(path)?;

        let mut magic = [0u8; 4];
        file.read_exact(&mut magic)?;

        if magic != [0xCB, 0xEC, 0xCB, 0xEC] {
            return Err(std::io::Error::new(
                ErrorKind::InvalidData,
                "File magic was incorrect.",
            ));
        }

        let mut header = [0u8; 2];
        file.read_exact(&mut header)?;

        let [orientation, compression] = header;

        if orientation != 0 {
            return Err(std::io::Error::new(
                ErrorKind::Other,
                "Only non-sorted orientation is supported",
            ));
        }

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

        let is_stream = cube_count == 0;

        if is_stream {
            println!("Loading streamed cubes...");
        } else {
            println!("Loading {cube_count} cubes...");
        }

        let alloc_count = Arc::new(AtomicUsize::new(0));
        let mut cubes = HashSet::new();

        let mut cubes_read = 0;
        loop {
            let next_cube = PolyCube::unpack_with(alloc_count.clone(), &mut file);

            let next_cube = match next_cube {
                Err(_) if is_stream => break,
                Err(e) => {
                    if cubes_read != cube_count {
                        panic!(
                            "Expected {cube_count} cubes, but failed to read after {cubes_read} cubes. Error: {e}"
                        );
                    }
                    break;
                }
                Ok(c) => c,
            };

            if !cubes.insert(next_cube.clone()) {
                println!("{next_cube}");
                panic!("Read non-unique cube {cubes_read} from file");
            }
            cubes_read += 1;
        }

        assert_eq!(cubes_read, cubes.len() as u64);

        Ok(cubes.into_iter().collect())
    }
}