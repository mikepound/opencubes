use super::CubeMap;

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum MatrixCol {
    XP,
    XN,
    YP,
    YN,
    ZP,
    ZN,
}

#[inline]
pub fn reverse_bits(mut n: u16, count: u32) -> u16 {
    let mut res = 0;
    for _ in 0..count {
        res = (res << 1) + (n & 1);
        n >>= 1;
    }
    res
}

#[inline]
pub fn rot_matrix(
    map: &CubeMap,
    rot: &mut CubeMap,
    x_col: MatrixCol,
    y_col: MatrixCol,
    z_col: MatrixCol,
) {
    match x_col {
        MatrixCol::XP => {
            for y in 0..=map.y as usize {
                for z in 0..=map.z as usize {
                    let row = map.cube_map[z * (map.y as usize + 1) + y];

                    let yv: usize = match y_col {
                        MatrixCol::YP => y,
                        MatrixCol::YN => map.y as usize - y,
                        MatrixCol::ZP => z,
                        MatrixCol::ZN => map.z as usize - z,
                        _ => panic!("impossible"),
                    };
                    let zv = match z_col {
                        MatrixCol::YP => y,
                        MatrixCol::YN => map.y as usize - y,
                        MatrixCol::ZP => z,
                        MatrixCol::ZN => map.z as usize - z,
                        _ => panic!("impossible"),
                    };
                    rot.cube_map[zv * (map.y as usize + 1) + yv] = row;
                }
            }
        }
        MatrixCol::XN => {
            for y in 0..=map.y as usize {
                for z in 0..=map.z as usize {
                    let row = map.cube_map[z * (map.y as usize + 1) + y];

                    let yv: usize = match y_col {
                        MatrixCol::YP => y,
                        MatrixCol::YN => map.y as usize - y,
                        MatrixCol::ZP => z,
                        MatrixCol::ZN => map.z as usize - z,
                        _ => panic!("impossible"),
                    };
                    let zv = match z_col {
                        MatrixCol::YP => y,
                        MatrixCol::YN => map.y as usize - y,
                        MatrixCol::ZP => z,
                        MatrixCol::ZN => map.z as usize - z,
                        _ => panic!("impossible"),
                    };
                    rot.cube_map[zv * (map.y as usize + 1) + yv] = reverse_bits(row, map.x + 1);
                }
            }
        }
        _ => {
            for x in 0..=map.x as usize {
                for y in 0..=map.y as usize {
                    for z in 0..=map.z as usize {
                        let v = map.get_block(x, y, z);
                        let xv = match x_col {
                            MatrixCol::XP => x,
                            MatrixCol::XN => map.x as usize - x,
                            MatrixCol::YP => y,
                            MatrixCol::YN => map.y as usize - y,
                            MatrixCol::ZP => z,
                            MatrixCol::ZN => map.z as usize - z,
                        };
                        let yv = match y_col {
                            MatrixCol::XP => x,
                            MatrixCol::XN => map.x as usize - x,
                            MatrixCol::YP => y,
                            MatrixCol::YN => map.y as usize - y,
                            MatrixCol::ZP => z,
                            MatrixCol::ZN => map.z as usize - z,
                        };
                        let zv = match z_col {
                            MatrixCol::XP => x,
                            MatrixCol::XN => map.x as usize - x,
                            MatrixCol::YP => y,
                            MatrixCol::YN => map.y as usize - y,
                            MatrixCol::ZP => z,
                            MatrixCol::ZN => map.z as usize - z,
                        };
                        rot.set_block_to(xv, yv, zv, v);
                    }
                }
            }
        }
    }
}

//xz rots + other rots for x==y==z
fn xyz_rots(map: &CubeMap) -> CubeMap {
    //xz rotations
    let mut rot = CubeMap {
        x: map.x,
        y: map.y,
        z: map.z,
        cube_map: [0; 36],
    };
    rot_matrix(map, &mut rot, MatrixCol::ZP, MatrixCol::YP, MatrixCol::XN);
    let mut res = *map;
    if &rot < &res {
        res = rot;
    }

    rot.clear();
    rot_matrix(map, &mut rot, MatrixCol::ZN, MatrixCol::YN, MatrixCol::XN);
    if &rot < &res {
        res = rot;
    }

    rot.clear();
    rot_matrix(map, &mut rot, MatrixCol::ZN, MatrixCol::YP, MatrixCol::XP);
    if &rot < &res {
        res = rot;
    }

    rot.clear();
    rot_matrix(map, &mut rot, MatrixCol::ZP, MatrixCol::YN, MatrixCol::XP);
    if &rot < &res {
        res = rot;
    }

    // Free rotations
    rot.clear();
    rot_matrix(map, &mut rot, MatrixCol::ZP, MatrixCol::XN, MatrixCol::YN);
    if &rot < &res {
        res = rot;
    }

    rot.clear();
    rot_matrix(map, &mut rot, MatrixCol::YP, MatrixCol::ZP, MatrixCol::XP);
    if &rot < &res {
        res = rot;
    }

    rot.clear();
    rot_matrix(map, &mut rot, MatrixCol::YN, MatrixCol::ZN, MatrixCol::XP);
    if &rot < &res {
        res = rot;
    }

    rot.clear();
    rot_matrix(map, &mut rot, MatrixCol::ZN, MatrixCol::XP, MatrixCol::YN);
    if &rot < &res {
        res = rot;
    }

    rot.clear();
    rot_matrix(map, &mut rot, MatrixCol::YP, MatrixCol::ZN, MatrixCol::XN);
    if &rot < &res {
        res = rot;
    }

    rot.clear();
    rot_matrix(map, &mut rot, MatrixCol::YN, MatrixCol::ZP, MatrixCol::XN);
    if &rot < &res {
        res = rot;
    }

    rot.clear();

    rot_matrix(map, &mut rot, MatrixCol::ZN, MatrixCol::XN, MatrixCol::YP);
    if &rot < &res {
        res = rot;
    }

    rot.clear();
    rot_matrix(map, &mut rot, MatrixCol::ZP, MatrixCol::XP, MatrixCol::YP);
    if &rot < &res {
        res = rot;
    }

    res
}

fn xy_rots(map: &CubeMap) -> CubeMap {
    let mut rot = CubeMap {
        x: map.x,
        y: map.y,
        z: map.z,
        cube_map: [0; 36],
    };
    rot_matrix(map, &mut rot, MatrixCol::YN, MatrixCol::XN, MatrixCol::ZN);
    let mut res = *map;
    if &rot < &res {
        res = rot;
    }

    rot.clear();
    rot_matrix(map, &mut rot, MatrixCol::YP, MatrixCol::XP, MatrixCol::ZN);
    if &rot < &res {
        res = rot;
    }

    rot.clear();
    rot_matrix(map, &mut rot, MatrixCol::YP, MatrixCol::XN, MatrixCol::ZP);
    if &rot < &res {
        res = rot;
    }

    rot.clear();
    rot_matrix(map, &mut rot, MatrixCol::YN, MatrixCol::XP, MatrixCol::ZP);
    if &rot < &res {
        res = rot;
    }

    res
}

fn yz_rots(map: &CubeMap) -> CubeMap {
    let mut rot = CubeMap {
        x: map.x,
        y: map.y,
        z: map.z,
        cube_map: [0; 36],
    };
    rot_matrix(map, &mut rot, MatrixCol::XN, MatrixCol::ZP, MatrixCol::YP);
    let mut res = *map;
    if &rot < &res {
        res = rot;
    }

    rot.clear();
    rot_matrix(map, &mut rot, MatrixCol::XN, MatrixCol::ZN, MatrixCol::YN);
    if &rot < &res {
        res = rot;
    }

    rot.clear();
    rot_matrix(map, &mut rot, MatrixCol::XP, MatrixCol::ZP, MatrixCol::YN);
    if &rot < &res {
        res = rot;
    }

    rot.clear();
    rot_matrix(map, &mut rot, MatrixCol::XP, MatrixCol::ZN, MatrixCol::YP);
    if &rot < &res {
        res = rot;
    }
    res
}

pub fn to_min_rot(map: &CubeMap) -> CubeMap {
    let mut rot = CubeMap {
        x: map.x,
        y: map.y,
        z: map.z,
        cube_map: [0; 36],
    };
    let mut res = *map;
    if map.x == map.y && map.x != 0 {
        res = xy_rots(map);
    }

    if map.y == map.z && map.y != 0 {
        let yz = yz_rots(map);
        if &yz < &res {
            res = yz;
        }
    }

    if map.x == map.y && map.y == map.z && map.x != 0 {
        let xyz = xyz_rots(map);
        if &xyz < &res {
            res = xyz;
        }
    }

    for i in 0..(((map.y + 1) * (map.z + 1)) as usize) {
        rot.cube_map[i] = map.cube_map[((map.y + 1) * (map.z + 1) - 1) as usize - i];
    }
    if &rot < &res {
        res = rot;
    }

    for z in 0..(map.z as usize + 1) {
        for y in 0..(map.y as usize + 1) {
            rot.cube_map[z * (map.y as usize + 1) + y] = reverse_bits(
                map.cube_map[(map.z as usize - z) * (map.y as usize + 1) + y],
                map.x + 1,
            );
        }
    }
    if &rot < &res {
        res = rot;
    }

    for z in 0..(map.z as usize + 1) {
        for y in 0..(map.y as usize + 1) {
            rot.cube_map[z * (map.y as usize + 1) + y] = reverse_bits(
                map.cube_map[z * (map.y as usize + 1) + (map.y as usize - y)],
                map.x + 1,
            );
        }
    }
    if &rot < &res {
        res = rot;
    }
    res
}
