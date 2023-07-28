pub mod expand;
pub mod rotate;

use std::{
    cmp::{max, min},
    iter::{Chain, Flatten},
    option::IntoIter,
};

use super::{pcube::RawPCube, rotation_reduced::rotate::MatrixCol, Dim, PolyCube};
use crate::polycubes::rotation_reduced::rotate::MatrixCol::*;

/// Polycube representation
/// stores up to 16 blocks (number of cubes normally implicit or seperate in program state)
/// well formed polycubes are a sorted list of coordinates low to high
/// cordinates are group of packed 5 bit unsigned integers '-ZZZZZYYYYYXXXXX'
#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, PartialOrd, Ord)]
pub struct CubeMapPos<const N: usize> {
    pub cubes: [u16; N],
}

/// Conversion function to assist with loading Polycubes from cache file to point-list implementation
/// Returned cubes may not be fully canonicalized (X >= Y >= Z guarenteed but not exact rotation)
impl<const N: usize> From<&RawPCube> for CubeMapPos<N> {
    fn from(src: &RawPCube) -> Self {
        let mut dst = CubeMapPos { cubes: [0; N] };
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
            (XP, YP, ZP, dim)
        } else if x >= z && z >= y {
            (
                XP,
                ZP,
                YN,
                Dim {
                    x: x - 1,
                    y: z - 1,
                    z: y - 1,
                },
            )
        } else if y >= x && x >= z {
            (
                YP,
                XP,
                ZN,
                Dim {
                    x: y - 1,
                    y: x - 1,
                    z: z - 1,
                },
            )
        } else if y >= z && z >= x {
            (
                YP,
                ZP,
                XP,
                Dim {
                    x: y - 1,
                    y: z - 1,
                    z: x - 1,
                },
            )
        } else if z >= x && x >= y {
            (
                ZN,
                XP,
                YN,
                Dim {
                    x: z - 1,
                    y: x - 1,
                    z: y - 1,
                },
            )
        } else if z >= y && y >= x {
            (
                ZN,
                YN,
                XP,
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
                        let cx = Self::map_coord(dx, dy, dz, &dim, x_col);
                        let cy = Self::map_coord(dx, dy, dz, &dim, y_col);
                        let cz = Self::map_coord(dx, dy, dz, &dim, z_col);
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

impl<const N: usize> From<&'_ CubeMapPos<N>> for RawPCube {
    fn from(src: &'_ CubeMapPos<N>) -> Self {
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

impl<const N: usize> From<RawPCube> for CubeMapPos<N> {
    fn from(value: RawPCube) -> Self {
        (&value).into()
    }
}

impl<const N: usize> From<CubeMapPos<N>> for RawPCube {
    fn from(value: CubeMapPos<N>) -> Self {
        (&value).into()
    }
}

/// Linearly scan backwards to insertion point overwrites end of slice
#[inline]
fn array_insert(val: u16, arr: &mut [u16]) {
    for i in 1..(arr.len()) {
        if arr[arr.len() - 1 - i] > val {
            arr[arr.len() - i] = arr[arr.len() - 1 - i];
        } else {
            arr[arr.len() - i] = val;
            return;
        }
    }
    arr[0] = val;
}

/// Moves contents of slice to index x+1, x==0 remains
#[inline]
fn array_shift(arr: &mut [u16]) {
    for i in 1..(arr.len()) {
        arr[arr.len() - i] = arr[arr.len() - 1 - i];
    }
}

impl<const N: usize> CubeMapPos<N> {
    pub fn new() -> Self {
        CubeMapPos { cubes: [0; N] }
    }

    #[inline]
    pub fn map_coord(x: u16, y: u16, z: u16, shape: &Dim, col: MatrixCol) -> u16 {
        match col {
            MatrixCol::XP => x,
            MatrixCol::XN => shape.x as u16 - x,
            MatrixCol::YP => y,
            MatrixCol::YN => shape.y as u16 - y,
            MatrixCol::ZP => z,
            MatrixCol::ZN => shape.z as u16 - z,
        }
    }

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

    fn is_continuous(&self, len: usize) -> bool {
        let start = self.cubes[0];
        let mut polycube2 = [start; 32];
        for i in 1..len {
            polycube2[i] = self.cubes[i];
        }
        let polycube = polycube2;
        //sets were actually slower even when no allocating
        let mut to_explore = [start; 32];
        let mut exp_head = 1;
        let mut exp_tail = 0;
        //to_explore[0] = start;
        while exp_head > exp_tail {
            let p = to_explore[exp_tail];
            exp_tail += 1;
            if p & 0x1f != 0 && !to_explore.contains(&(p - 1)) && polycube.contains(&(p - 1)) {
                to_explore[exp_head] = p - 1;
                // unsafe {*to_explore.get_unchecked_mut(exp_head) = p - 1;}
                exp_head += 1;
            }
            if p & 0x1f != 0x1f && !to_explore.contains(&(p + 1)) && polycube.contains(&(p + 1)) {
                to_explore[exp_head] = p + 1;
                // unsafe {*to_explore.get_unchecked_mut(exp_head) = p + 1;}
                exp_head += 1;
            }
            if (p >> 5) & 0x1f != 0
                && !to_explore.contains(&(p - (1 << 5)))
                && polycube.contains(&(p - (1 << 5)))
            {
                to_explore[exp_head] = p - (1 << 5);
                // unsafe {*to_explore.get_unchecked_mut(exp_head) = p - (1 << 5);}
                exp_head += 1;
            }
            if (p >> 5) & 0x1f != 0x1f
                && !to_explore.contains(&(p + (1 << 5)))
                && polycube.contains(&(p + (1 << 5)))
            {
                to_explore[exp_head] = p + (1 << 5);
                // unsafe {*to_explore.get_unchecked_mut(exp_head) = p + (1 << 5);}
                exp_head += 1;
            }
            if (p >> 10) & 0x1f != 0
                && !to_explore.contains(&(p - (1 << 10)))
                && polycube.contains(&(p - (1 << 10)))
            {
                to_explore[exp_head] = p - (1 << 10);
                // unsafe {*to_explore.get_unchecked_mut(exp_head) = p - (1 << 10);}
                exp_head += 1;
            }
            if (p >> 10) & 0x1f != 0x1f
                && !to_explore.contains(&(p + (1 << 10)))
                && polycube.contains(&(p + (1 << 10)))
            {
                to_explore[exp_head] = p + (1 << 10);
                // unsafe {*to_explore.get_unchecked_mut(exp_head) = p + (1 << 10);}
                exp_head += 1;
            }
        }
        exp_head == len
    }

    fn renormalize(&self, dim: &Dim, count: usize) -> (Self, Dim) {
        let mut dst = CubeMapPos::new();
        let x = dim.x;
        let y = dim.y;
        let z = dim.z;
        let (x_col, y_col, z_col, rdim) = if x >= y && y >= z {
            (XP, YP, ZP, Dim { x: x, y: y, z: z })
        } else if x >= z && z >= y {
            (XP, ZP, YN, Dim { x: x, y: z, z: y })
        } else if y >= x && x >= z {
            (YP, XP, ZN, Dim { x: y, y: x, z: z })
        } else if y >= z && z >= x {
            (YP, ZP, XP, Dim { x: y, y: z, z: x })
        } else if z >= x && x >= y {
            (ZN, XP, YN, Dim { x: z, y: x, z: y })
        } else if z >= y && y >= x {
            (ZN, YN, XP, Dim { x: z, y: y, z: x })
        } else {
            panic!("imposible dimension of shape {:?}", dim)
        };
        for (i, d) in self.cubes[0..count].iter().enumerate() {
            let dx = d & 0x1f;
            let dy = (d >> 5) & 0x1f;
            let dz = (d >> 10) & 0x1f;
            let cx = Self::map_coord(dx, dy, dz, &dim, x_col);
            let cy = Self::map_coord(dx, dy, dz, &dim, y_col);
            let cz = Self::map_coord(dx, dy, dz, &dim, z_col);
            let pack = ((cz << 10) | (cy << 5) | cx) as u16;
            dst.cubes[i] = pack;
            // unsafe {*dst.cubes.get_unchecked_mut(i) = pack;}
        }
        //dst.cubes.sort();
        (dst, rdim)
    }

    fn remove_cube(&self, point: usize, count: usize) -> (Self, Dim) {
        let mut min_corner = Dim {
            x: 0x1f,
            y: 0x1f,
            z: 0x1f,
        };
        let mut max_corner = Dim { x: 0, y: 0, z: 0 };
        let mut root_candidate = CubeMapPos::new();
        let mut candidate_ptr = 0;
        for i in 0..=count {
            if i != point {
                let pos = self.cubes[i];
                // let pos = unsafe {*exp.cubes.get_unchecked(i)};
                let x = pos as usize & 0x1f;
                let y = (pos as usize >> 5) & 0x1f;
                let z = (pos as usize >> 10) & 0x1f;
                min_corner.x = min(min_corner.x, x);
                min_corner.y = min(min_corner.y, y);
                min_corner.z = min(min_corner.z, z);
                max_corner.x = max(max_corner.x, x);
                max_corner.y = max(max_corner.y, y);
                max_corner.z = max(max_corner.z, z);
                root_candidate.cubes[candidate_ptr] = pos;
                // unsafe {*root_candidate.cubes.get_unchecked_mut(candidate_ptr) = pos;}
                candidate_ptr += 1;
            }
        }
        let offset = (min_corner.z << 10) | (min_corner.y << 5) | min_corner.x;
        for i in 0..count {
            root_candidate.cubes[i] -= offset as u16;
        }
        max_corner.x = max_corner.x - min_corner.x;
        max_corner.y = max_corner.y - min_corner.y;
        max_corner.z = max_corner.z - min_corner.z;
        (root_candidate, max_corner)
    }

    pub fn is_canonical_root(&self, count: usize, seed: &Self) -> bool {
        for sub_cube_id in 0..=count {
            let (mut root_candidate, mut dim) = self.remove_cube(sub_cube_id, count);
            if !root_candidate.is_continuous(count) {
                continue;
            }
            if dim.x < dim.y || dim.y < dim.z || dim.x < dim.z {
                let (rroot_candidate, rdim) = root_candidate.renormalize(&dim, count);
                root_candidate = rroot_candidate;
                dim = rdim;
                root_candidate.cubes[0..count].sort_unstable();
            }
            let mrp = root_candidate.to_min_rot_points(dim, count);
            if &mrp < seed {
                return false;
            }
        }
        true
    }
}

macro_rules! cube_map_pos_expand {
    ($name:ident, $dim:ident, $shift:literal) => {
        #[inline(always)]
        pub fn $name(self) -> impl Iterator<Item = Self> {
            struct Iter<const C: usize> {
                inner: PointListMeta<C>,
                i: usize,
                stored: Option<PointListMeta<C>>,
            }

            impl<'a, const C: usize> Iterator for Iter<C> {
                type Item = PointListMeta<C>;

                fn next(&mut self) -> Option<Self::Item> {
                    loop {
                        if let Some(stored) = self.stored.take() {
                            return Some(stored);
                        }

                        let i = self.i;

                        if i == self.inner.count {
                            return None;
                        }

                        self.i += 1;
                        let coord = *self.inner.point_list.cubes.get(i)?;

                        let plus = coord + (1 << $shift);
                        let minus = coord - (1 << $shift);

                        if !self.inner.point_list.cubes[(i + 1)..self.inner.count].contains(&plus) {
                            let mut new_shape = self.inner.dim;
                            let mut new_map = self.inner.point_list;

                            array_insert(plus, &mut new_map.cubes[i..=self.inner.count]);
                            new_shape.$dim =
                                max(new_shape.$dim, (((coord >> $shift) + 1) & 0x1f) as usize);

                            self.stored = Some(PointListMeta {
                                point_list: new_map,
                                dim: new_shape,
                                count: self.inner.count + 1,
                            });
                        }

                        let mut new_map = self.inner.point_list;
                        let mut new_shape = self.inner.dim;

                        // If the coord is out of bounds for $dim, shift everything
                        // over and create the cube at the out-of-bounds position.
                        // If it is in bounds, check if the $dim - 1 value already
                        // exists.
                        let insert_coord = if (coord >> $shift) & 0x1f != 0 {
                            if !self.inner.point_list.cubes[0..i].contains(&minus) {
                                minus
                            } else {
                                continue;
                            }
                        } else {
                            new_shape.$dim += 1;
                            for i in 0..self.inner.count {
                                new_map.cubes[i] += 1 << $shift;
                            }
                            coord
                        };

                        array_shift(&mut new_map.cubes[i..=self.inner.count]);
                        array_insert(insert_coord, &mut new_map.cubes[0..=i]);
                        return Some(PointListMeta {
                            point_list: new_map,
                            dim: new_shape,
                            count: self.inner.count + 1,
                        });
                    }
                }
            }

            Iter {
                inner: self,
                i: 0,
                stored: None,
            }
        }
    };
}

#[derive(Copy, Clone)]
pub struct PointListMeta<const N: usize> {
    pub point_list: CubeMapPos<N>,
    pub dim: Dim,
    pub count: usize,
}

impl<const N: usize> PointListMeta<N> {
    cube_map_pos_expand!(expand_x, x, 0);
    cube_map_pos_expand!(expand_y, y, 5);
    cube_map_pos_expand!(expand_z, z, 10);

    /// reduce number of expansions needing to be performed based on
    /// X >= Y >= Z constraint on Dim
    #[inline]
    #[must_use]
    fn do_expand(
        self,
    ) -> Chain<
        Chain<
            impl Iterator<Item = PointListMeta<N>>,
            Flatten<IntoIter<impl Iterator<Item = PointListMeta<N>>>>,
        >,
        Flatten<IntoIter<impl Iterator<Item = PointListMeta<N>>>>,
    > {
        let expand_ys = if self.dim.y < self.dim.x {
            Some(self.expand_y())
        } else {
            None
        };

        let expand_zs = if self.dim.z < self.dim.y {
            Some(self.expand_z())
        } else {
            None
        };

        self.expand_x()
            .chain(expand_ys.into_iter().flatten())
            .chain(expand_zs.into_iter().flatten())
    }

    /// perform the cube expansion for a given polycube
    /// if perform extra expansions for cases where the dimensions are equal as
    /// square sides may miss poly cubes otherwise
    #[inline]
    pub fn expand(&self) -> impl Iterator<Item = Self> + '_ {
        use MatrixCol::*;

        let z = if self.dim.x == self.dim.y && self.dim.x > 0 {
            let rotz = self
                .point_list
                .rot_matrix_points(self.dim, self.count, YN, XN, ZN, 1025);
            let rotz = PointListMeta {
                point_list: rotz,
                dim: self.dim,
                count: self.count,
            };
            Some(rotz.do_expand())
        } else {
            None
        };

        let y = if self.dim.y == self.dim.z && self.dim.y > 0 {
            let rotx = self
                .point_list
                .rot_matrix_points(self.dim, self.count, XN, ZP, YP, 1025);
            let rotx = PointListMeta {
                point_list: rotx,
                dim: self.dim,
                count: self.count,
            };
            Some(rotx.do_expand())
        } else {
            None
        };

        let x = if self.dim.x == self.dim.z && self.dim.x > 0 {
            let roty = self
                .point_list
                .rot_matrix_points(self.dim, self.count, ZP, YP, XN, 1025);
            let roty = PointListMeta {
                point_list: roty,
                dim: self.dim,
                count: self.count,
            };
            Some(roty.do_expand())
        } else {
            None
        };

        let seed = self.do_expand();

        let w = z
            .into_iter()
            .flatten()
            .chain(y.into_iter().flatten())
            .chain(x.into_iter().flatten())
            .chain(seed);
        w
    }
}

impl<const N: usize> From<RawPCube> for PointListMeta<N> {
    fn from(value: RawPCube) -> Self {
        let (x, y, z) = value.dims();
        let dim = Dim {
            x: x as usize,
            y: y as usize,
            z: z as usize,
        };
        let point_list = value.into();
        PointListMeta {
            point_list,
            dim,
            count: point_list.extrapolate_count(),
        }
    }
}

impl<const N: usize> From<PointListMeta<N>> for RawPCube {
    fn from(value: PointListMeta<N>) -> Self {
        value.point_list.into()
    }
}

impl<const N: usize> PolyCube for PointListMeta<N> {
    fn canonical_form(&self) -> Self {
        PointListMeta {
            point_list: self.point_list.to_min_rot_points(self.dims(), self.size()),
            count: self.count,
            dim: self.dim,
        }
    }

    fn size(&self) -> usize {
        self.count
    }

    fn dims(&self) -> Dim {
        self.dim
    }

    fn unique_expansions<'a>(&'a self) -> Box<dyn Iterator<Item = Self> + 'a> {
        Box::new(self.expand())
    }
}
