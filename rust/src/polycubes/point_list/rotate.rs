use std::cmp::min;

use crate::polycubes::rotation_reduced::rotate::MatrixCol;

use crate::polycubes::point_list::{CubeMapPos, Dim};

use MatrixCol::*;

// NOTE: this could technically be an `fn`, but iterating over the tuples
// slows down the program by 2x.
macro_rules ! rot_matrix_points {
    ($self:expr, $shape:expr, $count:expr, $res:expr, $(($x:expr, $y:expr, $z:expr),)*) => {
        $(
            $res = min($res, $self.rot_matrix_points($shape, $count, $x, $y, $z, $res.cubes[0]));
        )*
    }
}

macro_rules! def_rot_matrix_points {
    ($name:ident, $(($x:expr, $y:expr, $z:expr)),*) => {
        #[inline(always)]
        fn $name(&self, shape: &Dim, count: usize, res: &mut CubeMapPos<N>) {
            rot_matrix_points!(self, shape, count, *res, $(($x, $y, $z),)*);
        }
    };
}

impl<const N: usize> CubeMapPos<N> {
    #[inline]
    pub fn rot_matrix_points(
        &self,
        shape: &Dim,
        count: usize,
        x_col: MatrixCol,
        y_col: MatrixCol,
        z_col: MatrixCol,
        pmin: u16,
    ) -> CubeMapPos<N> {
        let mut res = CubeMapPos::new();
        let mut mmin = 1024;
        for (i, coord) in self.cubes[0..count].iter().enumerate() {
            let ix = coord & 0x1f;
            let iy = (coord >> 5) & 0x1f;
            let iz = (coord >> 10) & 0x1f;
            let dx = Self::map_coord(ix, iy, iz, shape, x_col);
            let dy = Self::map_coord(ix, iy, iz, shape, y_col);
            let dz = Self::map_coord(ix, iy, iz, shape, z_col);
            let v = (dz << 10) | (dy << 5) | dx;
            mmin = min(mmin, v);
            res.cubes[i] = v;
        }
        //shorcut sorting because sort used to be >55% of runtime
        if pmin < mmin {
            res.cubes[0] = 1 << 10;
            return res;
        }
        res.cubes[0..count].sort_unstable();
        res
    }

    def_rot_matrix_points!(
        xy_rots_points,
        (YN, XN, ZN),
        (YP, XP, ZN),
        (YP, XN, ZP),
        (YN, XP, ZP)
    );

    def_rot_matrix_points!(
        yz_rots_points,
        (XN, ZP, YP),
        (XN, ZN, YN),
        (XP, ZP, YN),
        (XP, ZN, YP)
    );

    def_rot_matrix_points!(
        xyz_rots_points,
        // xz
        (ZP, YP, XN),
        (ZN, YN, XN),
        (ZN, YP, XP),
        (ZP, YN, XP),
        // xyz
        (ZP, XN, YN),
        (YP, ZP, XP),
        (YN, ZN, XP),
        (ZN, XP, YN),
        (YP, ZN, XN),
        (YN, ZP, XN),
        (ZN, XN, YP),
        (ZP, XP, YP)
    );

    pub fn to_min_rot_points(&self, shape: &Dim, count: usize) -> CubeMapPos<N> {
        let mut res = *self;
        if shape.x == shape.y && shape.x != 0 {
            self.xy_rots_points(shape, count, &mut res);
        }

        if shape.y == shape.z && shape.y != 0 {
            self.yz_rots_points(shape, count, &mut res);
        }

        if shape.x == shape.y && shape.y == shape.z && shape.x != 0 {
            self.xyz_rots_points(shape, count, &mut res);
        }

        rot_matrix_points!(
            self,
            shape,
            count,
            res,
            (XP, YN, ZN),
            (XN, YP, ZN),
            (XN, YN, ZP),
        );

        res
    }
}
