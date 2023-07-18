use std::collections::HashSet;

use crate::PolyCube;

fn test_cube() -> PolyCube {
    let mut cube = PolyCube::new(2, 3, 4);

    cube.set(0, 0, 1).unwrap();
    cube.set(0, 0, 3).unwrap();

    cube.set(0, 1, 0).unwrap();
    cube.set(0, 1, 1).unwrap();
    cube.set(0, 1, 2).unwrap();
    cube.set(0, 1, 3).unwrap();

    cube.set(1, 0, 0).unwrap();
    cube.set(1, 0, 1).unwrap();

    cube.set(1, 1, 0).unwrap();
    cube.set(1, 1, 2).unwrap();

    cube.set(1, 2, 0).unwrap();
    cube.set(1, 2, 3).unwrap();

    cube
}

#[test]
pub fn from_vec3d() {
    let cube = test_cube();

    #[rustfmt::skip]
    let expected = PolyCube::from(vec![
        vec![
            vec![false, true,  false, true ],
            vec![true,  true,  true,  true ],
            vec![false, false, false, false],
        ],
        vec![
            vec![true,  true,  false, false],
            vec![true,  false, true,  false],
            vec![true,  false, false, true ],
        ]
    ]);

    assert_eq!(cube, expected);
}

#[test]
fn flip_0() {
    let mut cube = test_cube();

    #[rustfmt::skip]
    let expected = PolyCube::from(vec![
        vec![
            vec![true,  true,  false, false],
            vec![true,  false, true,  false],
            vec![true,  false, false, true ],
        ],
        vec![
            vec![false, true,  false, true ],
            vec![true,  true,  true,  true ],
            vec![false, false, false, false],
        ],
    ]);

    cube.flip(0);

    assert_eq!(cube, expected);
}

#[test]
fn flip_1() {
    let mut cube = test_cube();

    #[rustfmt::skip]
    let expected = PolyCube::from(vec![
        vec![
            vec![false, false, false, false],
            vec![true,  true,  true,  true ],
            vec![false, true,  false, true ],
        ],
        vec![
            vec![true,  false, false, true ],
            vec![true,  false, true,  false],
            vec![true,  true,  false, false],
        ],
    ]);

    cube.flip(1);

    assert_eq!(cube, expected);
}

#[test]
fn flip_2() {
    let mut cube = test_cube();

    #[rustfmt::skip]
    let expected = PolyCube::from(vec![
        vec![
            vec![true,  false, true,  false],
            vec![true,  true,  true,  true ],
            vec![false, false, false, false],
        ],
        vec![
            vec![false, false, true,  true ],
            vec![false, true,  false, true ],
            vec![true,  false, false, true ],
        ]
    ]);

    cube.flip(2);

    assert_eq!(cube, expected);
}

#[test]
#[should_panic]
fn flip_3() {
    let mut cube = test_cube();

    cube.flip(3);
}

#[test]
fn transpose_0_1() {
    let cube = test_cube();

    #[rustfmt::skip]
    let expected = PolyCube::from(vec![
        vec![
            vec![false, true,  false, true ],
            vec![true,  true,  false, false],
        ],
        vec![
            vec![true,  true,  true,  true ],
            vec![true,  false, true,  false],
        ],
        vec![
            vec![false, false, false, false],
            vec![true,  false, false, true],
        ]
    ]);

    assert_eq!(cube.transpose(1, 0, 2), expected);
}

#[test]
fn transpose_1_2() {
    let cube = test_cube();

    #[rustfmt::skip]
    let expected = PolyCube::from(vec![
        vec![
            vec![false, true, false],
            vec![true,  true, false],
            vec![false, true, false],
            vec![true,  true, false],
        ],
        vec![
            vec![true,  true,  true ],
            vec![true,  false, false],
            vec![false, true,  false],
            vec![false, false, true ],
        ],
    ]);

    assert_eq!(cube.transpose(0, 2, 1), expected);
}

#[test]
fn transpose_0_2() {
    let cube = test_cube();

    #[rustfmt::skip]
    let expected = PolyCube::from(vec![
        vec![
            vec![false, true ],
            vec![true,  true ],
            vec![false, true ],
        ],
        vec![
            vec![true,  true ],
            vec![true,  false],
            vec![false, false],
        ],
        vec![
            vec![false, false],
            vec![true,  true ],
            vec![false, false],
        ],
        vec![
            vec![true,  false],
            vec![true,  false],
            vec![false, true ],
        ],
    ]);

    assert_eq!(cube.transpose(2, 1, 0), expected);
}

#[test]
fn rot90_3_0_1() {
    let cube = test_cube();

    #[rustfmt::skip]
    let expected = PolyCube::from(vec![
        vec![
            vec![true,   true, false, false],
            vec![false,  true, false, true ],
        ],
        vec![
            vec![true,  false, true,  false],
            vec![true,  true,  true,  true ],
        ],
        vec![
            vec![true,  false, false, true ],
            vec![false, false, false, false],
        ],
    ]);

    assert_eq!(cube.rot90(3, (0, 1)), expected);
}

#[test]
fn crop() {
    #[rustfmt::skip]
    let input = PolyCube::from(vec![
        vec![
            vec![false, false, true, false], 
            vec![false, false, false, false],
        ],
        vec![
            vec![false, true, false, false], 
            vec![false, true, true, false],
        ],
    ]);

    #[rustfmt::skip]
    let expected = PolyCube::from(vec![
        vec![
            vec![false, true], 
            vec![false, false],
        ],
        vec![
            vec![true, false], 
            vec![true, true],
        ],
    ]);

    let input = input.crop();

    assert_eq!(input, expected);
}

/// The test cube should not have any non-unique rotations.
#[test]
pub fn all_are_unique() {
    let cube = test_cube();

    let all_rotations: HashSet<_> = cube.all_rotations().collect();

    assert_eq!(all_rotations.len(), 24);
}

#[test]
pub fn from_bytes() {
    #[rustfmt::skip]
    let expected = PolyCube::from(vec![
        vec![
            vec![true],
            vec![true],
            vec![false],
        ],
        vec![
            vec![true],
            vec![true],
            vec![false],
        ],
        vec![
            vec![false],
            vec![true],
            vec![false],
        ],
        vec![
            vec![false],
            vec![true],
            vec![true],
        ]
    ]);

    let bytes: Vec<u8> = vec![0x04, 0x03, 0x01, 0x9B, 0x0c];

    let from_bytes = PolyCube::unpack(&*bytes).unwrap();

    assert_eq!(expected, from_bytes);

    let mut to_bytes = Vec::new();
    from_bytes.pack(&mut to_bytes).unwrap();

    assert_eq!(bytes, to_bytes);
}
