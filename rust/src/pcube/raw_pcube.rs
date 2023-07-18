use std::io::{Read, Write};

#[derive(Debug, PartialEq)]
pub struct RawPCube {
    dim_1: u8,
    dim_2: u8,
    dim_3: u8,
    data: Vec<u8>,
}

impl RawPCube {
    pub fn new(dim_1: u8, dim_2: u8, dim_3: u8, data: &[u8]) -> Option<Self> {
        let len = (dim_1 as usize) * (dim_2 as usize) * (dim_3 as usize);
        let byte_len = (len + 7) / 8;

        if data.len() != byte_len {
            return None;
        }

        Some(Self {
            dim_1,
            dim_2,
            dim_3,
            data: data.to_vec(),
        })
    }

    fn index(&self, d1: u8, d2: u8, d3: u8) -> (usize, u8) {
        let [d1, d2, d3] = [d1 as usize, d2 as usize, d3 as usize];

        let d1 = d1 * (self.dim_2 as usize) * (self.dim_3 as usize);
        let d2 = d2 * (self.dim_3 as usize);
        let d3 = d3;

        let len = d1 + d2 + d3;

        let offset = len % 8;
        let mask = 1 << offset;

        (len / 8, mask)
    }

    pub fn get(&self, d1: u8, d2: u8, d3: u8) -> bool {
        let (index, mask) = self.index(d1, d2, d3);
        (self.data[index] & mask) == mask
    }

    pub fn set(&mut self, d1: u8, d2: u8, d3: u8, value: bool) {
        let (index, mask) = self.index(d1, d2, d3);
        if value {
            self.data[index] |= mask;
        } else {
            self.data[index] &= !mask;
        }
    }

    pub fn unpack(mut from: impl Read) -> std::io::Result<Self> {
        let mut xyz = [0u8; 3];
        from.read_exact(&mut xyz)?;

        let [dim_1, dim_2, dim_3] = xyz;
        let [d1, d2, d3] = [dim_1 as usize, dim_2 as usize, dim_3 as usize];

        let mut data = vec![0u8; ((d1 * d2 * d3) + 7) / 8];
        from.read_exact(&mut data)?;

        Ok(Self {
            dim_1,
            dim_2,
            dim_3,
            data,
        })
    }

    pub fn pack(&self, mut write: impl Write) -> std::io::Result<()> {
        write.write_all(&[self.dim_1, self.dim_2, self.dim_3])?;

        write.write_all(&self.data)?;

        Ok(())
    }
}

impl core::fmt::Display for RawPCube {
    // Format the polycube in a somewhat more easy to digest
    // format.
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut xy = String::new();

        for _ in 0..self.dim_3 {
            xy.push('-');
        }
        xy.push('\n');

        for x in 0..self.dim_1 {
            for y in 0..self.dim_2 {
                for z in 0..self.dim_3 {
                    if self.get(x, y, z) {
                        xy.push('1');
                    } else {
                        xy.push('0');
                    }
                }
                xy.push('\n');
            }

            for _ in 0..self.dim_3 {
                xy.push('-');
            }
            xy.push('\n');
        }

        write!(f, "{}", xy.trim_end())
    }
}

#[test]
pub fn from_bytes() {
    let expected = RawPCube {
        dim_1: 4,
        dim_2: 3,
        dim_3: 3,
        data: vec![0x9B, 0x0C, 0x0A, 0x0B, 0x0C],
    };

    macro_rules! assert_set {
        ($(($x:literal, $y:literal, $z:literal)),*) => {
            $(
                assert!(expected.get($x, $y, $z));
            )*
        };
    }

    println!("{expected}");

    assert_set!(
        (0, 0, 0),
        (0, 1, 0),
        (0, 0, 1),
        (0, 1, 1),
        (0, 1, 2),
        (0, 1, 3),
        (0, 2, 3)
    );

    let bytes: Vec<u8> = vec![0x04, 0x03, 0x01, 0x9B, 0x0c];

    let from_bytes = RawPCube::unpack(&*bytes).unwrap();

    assert_eq!(expected, from_bytes);

    let mut to_bytes = Vec::new();
    from_bytes.pack(&mut to_bytes).unwrap();

    assert_eq!(bytes, to_bytes);
}
