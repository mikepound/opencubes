use std::cmp::min;

use crate::polycubes::rotation_reduced::rotate::MatrixCol;

use crate::polycubes::point_list::{CubeMapPos, Dim};

use MatrixCol::*;

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

    #[inline]
    fn xy_rots_points(&self, shape: &Dim, count: usize, res: &mut CubeMapPos<N>) {
        *res = min(
            *res,
            self.rot_matrix_points(shape, count, YN, XN, ZN, res.cubes[0]),
        );

        *res = min(
            *res,
            self.rot_matrix_points(shape, count, YP, XP, ZN, res.cubes[0]),
        );

        *res = min(
            *res,
            self.rot_matrix_points(shape, count, YP, XN, ZP, res.cubes[0]),
        );

        *res = min(
            *res,
            self.rot_matrix_points(shape, count, YN, XP, ZP, res.cubes[0]),
        );
    }

    #[inline]
    fn yz_rots_points(&self, shape: &Dim, count: usize, res: &mut CubeMapPos<N>) {
        *res = min(
            *res,
            self.rot_matrix_points(shape, count, XN, ZP, YP, res.cubes[0]),
        );

        *res = min(
            *res,
            self.rot_matrix_points(shape, count, XN, ZN, YN, res.cubes[0]),
        );

        *res = min(
            *res,
            self.rot_matrix_points(shape, count, XP, ZP, YN, res.cubes[0]),
        );

        *res = min(
            *res,
            self.rot_matrix_points(shape, count, XP, ZN, YP, res.cubes[0]),
        );
    }

    #[inline]
    fn xyz_rots_points(&self, shape: &Dim, count: usize, res: &mut CubeMapPos<N>) {
        //xz
        *res = min(
            *res,
            self.rot_matrix_points(shape, count, ZP, YP, XN, res.cubes[0]),
        );

        *res = min(
            *res,
            self.rot_matrix_points(shape, count, ZN, YN, XN, res.cubes[0]),
        );

        *res = min(
            *res,
            self.rot_matrix_points(shape, count, ZN, YP, XP, res.cubes[0]),
        );

        *res = min(
            *res,
            self.rot_matrix_points(shape, count, ZP, YN, XP, res.cubes[0]),
        );

        //xyz
        *res = min(
            *res,
            self.rot_matrix_points(shape, count, ZP, XN, YN, res.cubes[0]),
        );

        *res = min(
            *res,
            self.rot_matrix_points(shape, count, YP, ZP, XP, res.cubes[0]),
        );

        *res = min(
            *res,
            self.rot_matrix_points(shape, count, YN, ZN, XP, res.cubes[0]),
        );

        *res = min(
            *res,
            self.rot_matrix_points(shape, count, ZN, XP, YN, res.cubes[0]),
        );

        *res = min(
            *res,
            self.rot_matrix_points(shape, count, YP, ZN, XN, res.cubes[0]),
        );

        *res = min(
            *res,
            self.rot_matrix_points(shape, count, YN, ZP, XN, res.cubes[0]),
        );

        *res = min(
            *res,
            self.rot_matrix_points(shape, count, ZN, XN, YP, res.cubes[0]),
        );

        *res = min(
            *res,
            self.rot_matrix_points(shape, count, ZP, XP, YP, res.cubes[0]),
        );
    }

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

        res = min(
            res,
            self.rot_matrix_points(shape, count, XP, YN, ZN, res.cubes[0]),
        );

        res = min(
            res,
            self.rot_matrix_points(shape, count, XN, YP, ZN, res.cubes[0]),
        );

        res = min(
            res,
            self.rot_matrix_points(shape, count, XN, YN, ZP, res.cubes[0]),
        );

        res
    }
}
