use std::cmp::max;

use opencubes::pcube::RawPCube;

use crate::rotations::{map_coord, MatrixCol};

/// Polycube representation
/// stores up to 16 blocks (number of cubes normally implicit or seperate in program state)
/// well formed polycubes are a sorted list of coordinates low to high
/// cordinates are group of packed 5 bit unsigned integers '-ZZZZZYYYYYXXXXX'
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, PartialOrd, Ord)]
pub struct CubeMapPos {
    pub cubes: [u16; 16],
}

/// Conversion function to assist with loading Polycubes from cache file to point-list implementation
/// Returned cubes may not be fully canonicalized (X >= Y >= Z guarenteed but not exact rotation)
impl From<&RawPCube> for CubeMapPos {
    fn from(src: &RawPCube) -> Self {
        let mut dst = CubeMapPos { cubes: [0; 16] };
        let (x, y, z) = src.dims();
        let x = x as usize;
        let y = y as usize;
        let z = z as usize;
        let dim = Dim {
            x: x as usize - 1,
            y: y as usize - 1,
            z: z as usize - 1,
        };

        //correction matrix to convert to canonical dimension. I dont like it but it works
        let (x_col, y_col, z_col, rdim) = if x >= y && y >= z {
            (MatrixCol::XP, MatrixCol::YP, MatrixCol::ZP, dim)
        } else if x >= z && z >= y {
            (
                MatrixCol::XP,
                MatrixCol::ZP,
                MatrixCol::YN,
                Dim {
                    x: x - 1,
                    y: z - 1,
                    z: y - 1,
                },
            )
        } else if y >= x && x >= z {
            (
                MatrixCol::YP,
                MatrixCol::XP,
                MatrixCol::ZN,
                Dim {
                    x: y - 1,
                    y: x - 1,
                    z: z - 1,
                },
            )
        } else if y >= z && z >= x {
            (
                MatrixCol::YP,
                MatrixCol::ZP,
                MatrixCol::XP,
                Dim {
                    x: y - 1,
                    y: z - 1,
                    z: x - 1,
                },
            )
        } else if z >= x && x >= y {
            (
                MatrixCol::ZN,
                MatrixCol::XP,
                MatrixCol::YN,
                Dim {
                    x: z - 1,
                    y: x - 1,
                    z: y - 1,
                },
            )
        } else if z >= y && y >= x {
            (
                MatrixCol::ZN,
                MatrixCol::YN,
                MatrixCol::XP,
                Dim {
                    x: z - 1,
                    y: y - 1,
                    z: x - 1,
                },
            )
        } else {
            panic!("imposible dimension of shape {:?}", dim)
        };

        let mut dst_index = 0;
        for dz in 0..z as u16 {
            for dy in 0..y as u16 {
                for dx in 0..x as u16 {
                    if src.get(dx as u8, dy as u8, dz as u8) {
                        let cx = map_coord(dx, dy, dz, &dim, x_col);
                        let cy = map_coord(dx, dy, dz, &dim, y_col);
                        let cz = map_coord(dx, dy, dz, &dim, z_col);
                        if cx > rdim.x as u16 || cy > rdim.y as u16 || cz > rdim.z as u16 {
                            panic!("illegal block place {}, {}, {} {:?}", cx, cy, cz, dim)
                        }
                        let pack = ((cz << 10) | (cy << 5) | cx) as u16;
                        dst.cubes[dst_index] = pack;
                        dst_index += 1;
                    }
                }
            }
        }
        dst
    }
}

impl From<&'_ CubeMapPos> for RawPCube {
    fn from(src: &'_ CubeMapPos) -> Self {
        //cube is sorted numerically and then has trailing zeros
        let count = src.extrapolate_count();
        let dim = src.extrapolate_dim();

        let mut dst = RawPCube::new_empty(dim.x as u8 + 1, dim.y as u8 + 1, dim.z as u8 + 1);
        for p in src.cubes[0..count].iter() {
            let ix = *p & 0x1f;
            let iy = (*p >> 5) & 0x1f;
            let iz = (*p >> 10) & 0x1f;
            dst.set(ix as u8, iy as u8, iz as u8, true);
        }
        dst
    }
}

impl From<RawPCube> for CubeMapPos {
    fn from(value: RawPCube) -> Self {
        value.into()
    }
}

impl From<CubeMapPos> for RawPCube {
    fn from(value: CubeMapPos) -> Self {
        value.into()
    }
}

impl CubeMapPos {
    pub fn extrapolate_count(&self) -> usize {
        let mut count = 1;
        while self.cubes[count] > self.cubes[count - 1] {
            count += 1;
        }
        count
    }
    pub fn extrapolate_dim(&self) -> Dim {
        let count = self.extrapolate_count();
        let mut dim = Dim { x: 0, y: 0, z: 0 };
        for p in self.cubes[0..count].iter() {
            let ix = *p & 0x1f;
            let iy = (*p >> 5) & 0x1f;
            let iz = (*p >> 10) & 0x1f;
            dim.x = max(dim.x, ix as usize);
            dim.y = max(dim.y, iy as usize);
            dim.z = max(dim.z, iz as usize);
        }
        dim
    }
}

/// Partial Polycube representation for storage
/// the first block is stored seperately and used as a key to access the set
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, PartialOrd, Ord)]
pub struct CubeMapPosPart {
    pub cubes: [u16; 15],
}

/// the "Dimension" or "Shape" of a poly cube
/// defines the maximum bounds of a polycube
/// X >= Y >= Z for efficiency reasons reducing the number of rotations needing to be performed
/// stores len() - 1 for each dimension so the unit cube has a size of (0, 0, 0)
/// and the 2x1x1 starting seed has a dimension of (1, 0, 0)
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, PartialOrd, Ord)]
pub struct Dim {
    pub x: usize,
    pub y: usize,
    pub z: usize,
}

#[cfg(not(feature = "size16"))]
pub type CubeRow = u32;
#[cfg(feature = "size16")]
pub type CubeRow = u16;

//CubeRow is an integer type either u16 or u32 based on flags
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct CubeMap {
    pub x: u32, //max x index (len(xs) - 1)
    pub y: u32, //max y index (len(ys) - 1)
    pub z: u32, //max z index (len(zs) - 1)
    pub cube_map: [CubeRow; 6 * 6],
}

impl PartialOrd for CubeMap {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match self.cube_map.partial_cmp(&other.cube_map) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.x.partial_cmp(&other.x) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        match self.y.partial_cmp(&other.y) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.z.partial_cmp(&other.z)
    }
}

impl CubeMap {
    /// returns 1 if it block at xyz exists
    /// returns 0 if it doesnt
    pub fn get_block(&self, x: usize, y: usize, z: usize) -> CubeRow {
        (self.cube_map[z * (self.y as usize + 1) + y] >> x) & 1
    }
    #[cfg(feature = "smallset")]
    /// sets the block at xyz to exist
    pub fn set_block(&mut self, x: usize, y: usize, z: usize) {
        self.cube_map[z * (self.y as usize + 1) + y] |= 1 << x;
    }
    /// set a block to the bit v
    /// IMORTANT: Sets, does not unset, performs an | on the vale will never clear even on set v = 0
    pub fn set_block_to(&mut self, x: usize, y: usize, z: usize, v: CubeRow) {
        self.cube_map[z * (self.y as usize + 1) + y] |= v << x;
    }
    pub fn clear(&mut self) {
        for i in 0..((self.z as usize + 1) * (self.y as usize + 1)) {
            self.cube_map[i] = 0;
        }
    }
    #[cfg(feature = "diagnostics")]
    /// ensure expected number of cubes are set, only used as an integrity check
    pub fn count_cubes(&self) -> usize {
        let mut c = 0;
        for i in 0..36 {
            let mut x = self.cube_map[i];
            while x > 0 {
                c += x as usize & 1;
                x >>= 1;
            }
        }
        c
    }
    #[cfg(feature = "diagnostics")]
    /// ensure no blocks are set outside expected area, only used as an integrity check
    pub fn validate_bounds(&self) -> bool {
        for x in (self.x + 1)..MAX_X as u32 {
            for y in 0..=self.y {
                for z in 0..=self.z {
                    if self.get_block(x as usize, y as usize, z as usize) == 1 {
                        return false;
                    }
                }
            }
        }
        for i in ((self.z as usize + 1) * (self.y as usize + 1))..36 {
            if self.cube_map[i] != 0 {
                return false;
            }
        }
        true
    }

    #[cfg(feature = "diagnostics")]
    /// find an existing block to seed continuity check
    fn find_a_block(&self) -> Dim {
        for y in 0..=self.y {
            for z in 0..=self.z {
                let mut x = 0;
                let mut row = self.cube_map[(z * (self.y + 1) + y) as usize];
                if row != 0 {
                    while row > 0 {
                        if row & 1 == 1 {
                            return Dim {
                                x: x as usize,
                                y: y as usize,
                                z: z as usize,
                            };
                        }
                        x += 1;
                        row >>= 1;
                    }
                }
            }
        }
        panic!("{:?} empty", self);
    }

    #[cfg(feature = "diagnostics")]
    /// ensure all blocks are connected, only used as an integrity check
    pub fn validate_continuity(&self) -> bool {
        let mut to_visit = HashSet::new();
        let mut map = *self;
        let start = self.find_a_block();
        to_visit.insert(start);
        while let Some(p) = to_visit.iter().next() {
            let p = *p;
            to_visit.remove(&p);
            map.cube_map[p.z * (map.y as usize + 1) + p.y] &= !(1 << p.x);
            if p.x > 0 && (map.cube_map[p.z * (map.y as usize + 1) + p.y] >> (p.x - 1)) & 1 == 1 {
                to_visit.insert(Dim {
                    x: p.x - 1,
                    y: p.y,
                    z: p.z,
                });
            }
            if p.x < map.x as usize
                && (map.cube_map[p.z * (map.y as usize + 1) + p.y] >> (p.x + 1)) & 1 == 1
            {
                to_visit.insert(Dim {
                    x: p.x + 1,
                    y: p.y,
                    z: p.z,
                });
            }
            if p.y > 0 && (map.cube_map[p.z * (map.y as usize + 1) + (p.y - 1)] >> p.x) & 1 == 1 {
                to_visit.insert(Dim {
                    x: p.x,
                    y: p.y - 1,
                    z: p.z,
                });
            }
            if p.y < map.y as usize
                && (map.cube_map[p.z * (map.y as usize + 1) + (p.y + 1)] >> p.x) & 1 == 1
            {
                to_visit.insert(Dim {
                    x: p.x,
                    y: p.y + 1,
                    z: p.z,
                });
            }
            if p.z > 0 && (map.cube_map[(p.z - 1) * (map.y as usize + 1) + p.y] >> p.x) & 1 == 1 {
                to_visit.insert(Dim {
                    x: p.x,
                    y: p.y,
                    z: p.z - 1,
                });
            }
            if p.z < map.z as usize
                && (map.cube_map[(p.z + 1) * (map.y as usize + 1) + p.y] >> p.x) & 1 == 1
            {
                to_visit.insert(Dim {
                    x: p.x,
                    y: p.y,
                    z: p.z + 1,
                });
            }
        }
        for i in 0..((map.y + 1) * (map.z + 1)) {
            if map.cube_map[i as usize] != 0 {
                return false;
            }
        }
        true
    }
}
