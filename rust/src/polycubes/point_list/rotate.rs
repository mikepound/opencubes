use std::cmp::min;

use crate::polycubes::rotation_reduced::rotate::MatrixCol;

use crate::polycubes::point_list::{CubeMapPos, Dim};

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

#[inline]
pub fn rot_matrix_points<const N: usize>(
    map: &CubeMapPos<N>,
    shape: &Dim,
    count: usize,
    x_col: MatrixCol,
    y_col: MatrixCol,
    z_col: MatrixCol,
    pmin: u16,
) -> CubeMapPos<N> {
    let mut res = CubeMapPos::new();
    let mut mmin = 1024;
    for (i, coord) in map.cubes[0..count].iter().enumerate() {
        let ix = coord & 0x1f;
        let iy = (coord >> 5) & 0x1f;
        let iz = (coord >> 10) & 0x1f;
        let dx = map_coord(ix, iy, iz, shape, x_col);
        let dy = map_coord(ix, iy, iz, shape, y_col);
        let dz = map_coord(ix, iy, iz, shape, z_col);
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
fn xy_rots_points<const N: usize>(
    map: &CubeMapPos<N>,
    shape: &Dim,
    count: usize,
    res: &mut CubeMapPos<N>,
) {
    *res = min(
        *res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::YN,
            MatrixCol::XN,
            MatrixCol::ZN,
            res.cubes[0],
        ),
    );

    *res = min(
        *res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::YP,
            MatrixCol::XP,
            MatrixCol::ZN,
            res.cubes[0],
        ),
    );

    *res = min(
        *res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::YP,
            MatrixCol::XN,
            MatrixCol::ZP,
            res.cubes[0],
        ),
    );

    *res = min(
        *res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::YN,
            MatrixCol::XP,
            MatrixCol::ZP,
            res.cubes[0],
        ),
    );
}

#[inline]
fn yz_rots_points<const N: usize>(
    map: &CubeMapPos<N>,
    shape: &Dim,
    count: usize,
    res: &mut CubeMapPos<N>,
) {
    *res = min(
        *res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::XN,
            MatrixCol::ZP,
            MatrixCol::YP,
            res.cubes[0],
        ),
    );

    *res = min(
        *res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::XN,
            MatrixCol::ZN,
            MatrixCol::YN,
            res.cubes[0],
        ),
    );

    *res = min(
        *res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::XP,
            MatrixCol::ZP,
            MatrixCol::YN,
            res.cubes[0],
        ),
    );

    *res = min(
        *res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::XP,
            MatrixCol::ZN,
            MatrixCol::YP,
            res.cubes[0],
        ),
    );
}

#[inline]
fn xyz_rots_points<const N: usize>(
    map: &CubeMapPos<N>,
    shape: &Dim,
    count: usize,
    res: &mut CubeMapPos<N>,
) {
    //xz
    *res = min(
        *res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::ZP,
            MatrixCol::YP,
            MatrixCol::XN,
            res.cubes[0],
        ),
    );

    *res = min(
        *res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::ZN,
            MatrixCol::YN,
            MatrixCol::XN,
            res.cubes[0],
        ),
    );

    *res = min(
        *res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::ZN,
            MatrixCol::YP,
            MatrixCol::XP,
            res.cubes[0],
        ),
    );

    *res = min(
        *res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::ZP,
            MatrixCol::YN,
            MatrixCol::XP,
            res.cubes[0],
        ),
    );

    //xyz
    *res = min(
        *res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::ZP,
            MatrixCol::XN,
            MatrixCol::YN,
            res.cubes[0],
        ),
    );

    *res = min(
        *res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::YP,
            MatrixCol::ZP,
            MatrixCol::XP,
            res.cubes[0],
        ),
    );

    *res = min(
        *res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::YN,
            MatrixCol::ZN,
            MatrixCol::XP,
            res.cubes[0],
        ),
    );

    *res = min(
        *res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::ZN,
            MatrixCol::XP,
            MatrixCol::YN,
            res.cubes[0],
        ),
    );

    *res = min(
        *res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::YP,
            MatrixCol::ZN,
            MatrixCol::XN,
            res.cubes[0],
        ),
    );

    *res = min(
        *res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::YN,
            MatrixCol::ZP,
            MatrixCol::XN,
            res.cubes[0],
        ),
    );

    *res = min(
        *res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::ZN,
            MatrixCol::XN,
            MatrixCol::YP,
            res.cubes[0],
        ),
    );

    *res = min(
        *res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::ZP,
            MatrixCol::XP,
            MatrixCol::YP,
            res.cubes[0],
        ),
    );
}

pub fn to_min_rot_points<const N: usize>(
    map: &CubeMapPos<N>,
    shape: &Dim,
    count: usize,
) -> CubeMapPos<N> {
    let mut res = *map;
    if shape.x == shape.y && shape.x != 0 {
        xy_rots_points(map, shape, count, &mut res);
    }

    if shape.y == shape.z && shape.y != 0 {
        yz_rots_points(map, shape, count, &mut res);
    }

    if shape.x == shape.y && shape.y == shape.z && shape.x != 0 {
        xyz_rots_points(map, shape, count, &mut res);
    }

    res = min(
        res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::XP,
            MatrixCol::YN,
            MatrixCol::ZN,
            res.cubes[0],
        ),
    );

    res = min(
        res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::XN,
            MatrixCol::YP,
            MatrixCol::ZN,
            res.cubes[0],
        ),
    );

    res = min(
        res,
        rot_matrix_points(
            map,
            shape,
            count,
            MatrixCol::XN,
            MatrixCol::YN,
            MatrixCol::ZP,
            res.cubes[0],
        ),
    );

    res
}
