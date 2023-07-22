use core::cmp::Ordering;
use std::io::{Read, Write};

/// A PolyCube read directly from a pcube file.
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct RawPCube {
    dim_1: u8,
    dim_2: u8,
    dim_3: u8,
    data: Vec<u8>,
}

impl RawPCube {
    /// Compare two [`RawPCube`] of the same form in different
    /// rotations to determine their canonical ordering.
    pub fn canonical_cmp(&self, other: &Self) -> Ordering {
        if self.dim_1.cmp(&other.dim_1) == Ordering::Greater {
            return Ordering::Greater;
        }

        if self.dim_2.cmp(&other.dim_2) == Ordering::Greater {
            return Ordering::Greater;
        }

        if self.dim_3.cmp(&other.dim_3) == Ordering::Greater {
            return Ordering::Greater;
        }

        self.data.cmp(&other.data)
    }

    pub fn dims(&self) -> (u8, u8, u8) {
        (self.dim_1, self.dim_2, self.dim_3)
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }

    pub fn new(dim_1: u8, dim_2: u8, dim_3: u8, data: Vec<u8>) -> Option<Self> {
        let len = (dim_1 as usize) * (dim_2 as usize) * (dim_3 as usize);
        let byte_len = (len + 7) / 8;

        if data.len() != byte_len {
            return None;
        }

        Some(Self {
            dim_1,
            dim_2,
            dim_3,
            data,
        })
    }

    pub fn new_empty(dim_1: u8, dim_2: u8, dim_3: u8) -> Self {
        let len = (dim_1 as usize) * (dim_2 as usize) * (dim_3 as usize);
        let byte_len = (len + 7) / 8;

        let data = vec![0; byte_len];

        Self {
            dim_1,
            dim_2,
            dim_3,
            data,
        }
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

        for z in 0..self.dim_1 {
            for y in 0..self.dim_2 {
                for x in 0..self.dim_3 {
                    if self.get(z, y, x) {
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
    let data = [0x9B, 0x0c, 0xFF, 0x00, 0x33, 0xAB, 0xBA, 0x00];

    let expected = RawPCube {
        dim_1: 4,
        dim_2: 3,
        dim_3: 5,
        data: data.to_vec(),
    };

    let input_data: Vec<_> = [0x04, 0x03, 0x05]
        .into_iter()
        .chain(data.into_iter())
        .collect();

    let cube = RawPCube::unpack(&*input_data).unwrap();

    println!("{}", cube);

    assert_eq!(expected, cube);

    macro_rules! assert_set {
        ([
            $([
                $([
                    $($v:literal$(,)?)*
                ],)*
            ])*
        ]) => {
            #[allow(unused_assignments)]
            {
            let mut d1 = 0;
            let mut d2 = 0;
            let mut d3 = 0;
            $(
                $(
                    $(
                        let v = cube.get(d1, d2, d3) as u8;
                        assert_eq!(v, $v, "{d1}, {d2}, {d3}");
                        d3 += 1;
                    )*
                    d2 += 1;
                    d3 = 0;
                )*
                d1 += 1;
                d2 = 0;
                d3 = 0;
            )*
            }
        };
    }

    /*
       -----
       11011
       00100
       11000
       -----
       01111
       11110
       00000
       -----
       00110
       01100
       11010
       -----
       10101
       01110
       10000
       -----
    */

    assert_set!([
        [
            [1, 1, 0, 1, 1],
            [0, 0, 1, 0, 0],
            [1, 1, 0, 0, 0],
        ]
        [
            [0, 1, 1, 1, 1],
            [1, 1, 1, 1, 0],
            [0, 0, 0, 0, 0],
        ]
        [
            [0, 0, 1, 1, 0],
            [0, 1, 1, 0, 0],
            [1, 1, 0, 1, 0],
        ]
        [
            [1, 0, 1, 0, 1],
            [0, 1, 1, 1, 0],
            [1, 0, 0, 0, 0],
        ]
    ]);

    let mut out = Vec::new();

    cube.pack(&mut out).unwrap();

    assert_eq!(out, input_data);
}
