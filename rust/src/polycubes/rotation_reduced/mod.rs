pub mod expand;
pub mod rotate;

use hashbrown::HashSet;

use super::Dim;

//CubeRow is an integer type either u16 or u32 based on flags
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug)]
pub struct CubeMap {
    pub x: u32, //max x index (len(xs) - 1)
    pub y: u32, //max y index (len(ys) - 1)
    pub z: u32, //max z index (len(zs) - 1)
    pub cube_map: [u16; 6 * 6],
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
    pub fn get_block(&self, x: usize, y: usize, z: usize) -> u16 {
        (self.cube_map[z * (self.y as usize + 1) + y] >> x) & 1
    }

    /// sets the block at xyz to exist
    pub fn set_block(&mut self, x: usize, y: usize, z: usize) {
        self.cube_map[z * (self.y as usize + 1) + y] |= 1 << x;
    }
    /// set a block to the bit v
    /// IMORTANT: Sets, does not unset, performs an | on the vale will never clear even on set v = 0
    pub fn set_block_to(&mut self, x: usize, y: usize, z: usize, v: u16) {
        self.cube_map[z * (self.y as usize + 1) + y] |= v << x;
    }
    pub fn clear(&mut self) {
        for i in 0..((self.z as usize + 1) * (self.y as usize + 1)) {
            self.cube_map[i] = 0;
        }
    }

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
    /// ensure no blocks are set outside expected area, only used as an integrity check
    pub fn validate_bounds(&self) -> bool {
        for x in (self.x + 1)..16 as u32 {
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
