use std::cmp::max;

use hashbrown::HashSet;

use crate::{
    pointlist::{array_insert, array_shift},
    polycube_reps::{CubeMapPos, Dim},
    rotations::{rot_matrix_points, to_min_rot_points, MatrixCol},
};

pub struct MapStore<const N: usize> {
    inner: HashSet<CubeMapPos<N>>,
}

macro_rules! cube_map_pos_expand {
    ($name:ident, $dim:ident, $shift:literal) => {
        #[inline(always)]
        pub fn $name<'a>(
            &'a self,
            shape: &'a Dim,
            count: usize,
        ) -> impl Iterator<Item = (Dim, usize, Self)> + 'a {
            struct Iter<'a, const C: usize> {
                inner: &'a CubeMapPos<C>,
                shape: &'a Dim,
                count: usize,
                i: usize,
                stored: Option<(Dim, usize, CubeMapPos<C>)>,
            }

            impl<'a, const C: usize> Iterator for Iter<'a, C> {
                type Item = (Dim, usize, CubeMapPos<C>);

                fn next(&mut self) -> Option<Self::Item> {
                    loop {
                        if let Some(stored) = self.stored.take() {
                            return Some(stored);
                        }

                        let i = self.i;

                        if i == self.count {
                            return None;
                        }

                        self.i += 1;
                        let coord = *self.inner.cubes.get(i)?;

                        let plus = coord + (1 << $shift);
                        let minus = coord - (1 << $shift);

                        if !self.inner.cubes[(i + 1)..self.count].contains(&plus) {
                            let mut new_shape = *self.shape;
                            let mut new_map = *self.inner;

                            array_insert(plus, &mut new_map.cubes[i..=self.count]);
                            new_shape.$dim =
                                max(new_shape.$dim, (((coord >> $shift) + 1) & 0x1f) as usize);

                            self.stored = Some((new_shape, self.count + 1, new_map));
                        }

                        let mut new_map = *self.inner;
                        let mut new_shape = *self.shape;

                        // If the coord is out of bounds for $dim, shift everything
                        // over and create the cube at the out-of-bounds position.
                        // If it is in bounds, check if the $dim - 1 value already
                        // exists.
                        let insert_coord = if (coord >> $shift) & 0x1f != 0 {
                            if !self.inner.cubes[0..i].contains(&minus) {
                                minus
                            } else {
                                continue;
                            }
                        } else {
                            new_shape.$dim += 1;
                            for i in 0..self.count {
                                new_map.cubes[i] += 1 << $shift;
                            }
                            coord
                        };

                        array_shift(&mut new_map.cubes[i..=self.count]);
                        array_insert(insert_coord, &mut new_map.cubes[0..=i]);
                        return Some((new_shape, self.count + 1, new_map));
                    }
                }
            }

            Iter {
                inner: self,
                shape,
                count,
                i: 0,
                stored: None,
            }
        }
    };
}

impl<const N: usize> CubeMapPos<N> {
    cube_map_pos_expand!(expand_x, x, 0);
    cube_map_pos_expand!(expand_y, y, 5);
    cube_map_pos_expand!(expand_z, z, 10);
}

impl<const N: usize> MapStore<N> {
    pub fn new() -> Self {
        Self {
            inner: HashSet::new(),
        }
    }

    /// helper function to not duplicate code for canonicalising polycubes
    /// and storing them in the hashset
    fn insert_map(&mut self, dim: &Dim, map: &CubeMapPos<N>, count: usize) {
        if !self.inner.contains(map) {
            let map = to_min_rot_points(map, dim, count);
            self.inner.insert(map);
        }
    }

    /// reduce number of expansions needing to be performed based on
    /// X >= Y >= Z constraint on Dim
    #[inline]
    fn do_cube_expansion(&mut self, seed: &CubeMapPos<N>, shape: &Dim, count: usize) {
        let expand_ys = if shape.y < shape.x {
            Some(seed.expand_y(shape, count))
        } else {
            None
        };

        let expand_zs = if shape.z < shape.y {
            Some(seed.expand_z(shape, count))
        } else {
            None
        };

        seed.expand_x(shape, count)
            .chain(expand_ys.into_iter().flatten())
            .chain(expand_zs.into_iter().flatten())
            .for_each(|(dim, new_count, map)| self.insert_map(&dim, &map, new_count));
    }

    /// perform the cube expansion for a given polycube
    /// if perform extra expansions for cases where the dimensions are equal as
    /// square sides may miss poly cubes otherwise
    #[inline]
    fn expand_cube_map(&mut self, seed: &CubeMapPos<N>, shape: &Dim, count: usize) {
        use MatrixCol::*;

        if shape.x == shape.y && shape.x > 0 {
            let rotz = rot_matrix_points(seed, shape, count, YN, XN, ZN, 1025);
            self.do_cube_expansion(&rotz, shape, count);
        }

        if shape.y == shape.z && shape.y > 0 {
            let rotx = rot_matrix_points(seed, shape, count, XN, ZP, YP, 1025);
            self.do_cube_expansion(&rotx, shape, count);
        }
        if shape.x == shape.z && shape.x > 0 {
            let roty = rot_matrix_points(seed, shape, count, ZP, YP, XN, 1025);
            self.do_cube_expansion(&roty, shape, count);
        }

        self.do_cube_expansion(seed, shape, count);
    }

    /// Calculate the amount of canonical children of size `target`
    /// that polycube `seed` of size `count` has.
    ///
    /// This function does not store variants of the polycubes that
    /// it enumerates, it just keeps the count. This way, memory
    /// overhead is minimal.
    // TODO: improve this name once we unify this and pointslist
    pub fn enumerate_canonical_children_min_mem(
        seed: &CubeMapPos<N>,
        count: usize,
        target: usize,
    ) -> usize {
        let mut map = Self::new();
        let shape = seed.extrapolate_dim();

        let seed = to_min_rot_points(seed, &shape, count);
        let shape = seed.extrapolate_dim();

        map.expand_cube_map(&seed, &shape, count);

        map.inner
            .retain(|child| child.is_canonical_root(count, &seed));

        if count + 1 == target {
            map.inner.len()
        } else {
            map.inner
                .iter()
                .map(|child| Self::enumerate_canonical_children_min_mem(child, count + 1, target))
                .sum()
        }
    }
}
