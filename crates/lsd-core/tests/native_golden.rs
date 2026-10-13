//! Numeric golden observations from actual original PS1 instruction execution.
use lsd_core::{AnimationWord, CellBounds, PsxRng};

fn rows(text: &str) -> impl Iterator<Item = Vec<i64>> + '_ {
    text.lines().skip(1).map(|line| {
        line.split(',')
            .map(|field| field.parse().unwrap())
            .collect()
    })
}

#[test]
fn rng_matches_native_seed_streams_and_internal_states() {
    let mut rng = PsxRng::new(0);
    let mut count = 0;
    for row in rows(include_str!("fixtures/rng.csv")) {
        if row[1] == 1 {
            rng.reseed(row[0] as u32);
        }
        assert_eq!(rng.next_u15(), row[3] as u16, "{row:?}");
        assert_eq!(rng.state(), row[2] as u32, "{row:?}");
        count += 1;
    }
    assert_eq!(count, 128);
}

#[test]
fn animation_fields_match_native_decoder_observations() {
    let mut count = 0;
    for row in rows(include_str!("fixtures/animation.csv")) {
        let result = AnimationWord::decode(row[0] as u32);
        assert_eq!(
            [
                result.part_type,
                result.kind,
                result.flags,
                result.word_count
            ],
            [row[1] as u8, row[2] as u8, row[3] as u8, row[4] as u8],
            "{row:?}"
        );
        count += 1;
    }
    assert_eq!(count, 16);
}

#[test]
fn signed_cell_bounds_match_native_null_and_boundary_observations() {
    let mut count = 0;
    for row in rows(include_str!("fixtures/bounds.csv")) {
        let inside = row[0] != 0
            && CellBounds {
                min_x: row[1] as i16,
                min_z: row[2] as i16,
                max_x: row[3] as i32,
                max_z: row[4] as i32,
            }
            .contains([row[5] as i8, row[6] as i8]);
        assert_eq!(!inside, row[7] != 0, "{row:?}");
        count += 1;
    }
    assert_eq!(count, 2436);
}
