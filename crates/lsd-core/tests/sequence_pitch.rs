use lsd_core::SequencePitchTable;
use serde_json::Value;

#[test]
fn music_pitch_matches_original_signed_remainders_correction_clamp_and_fine_carry() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/sequence-pitch.json")).unwrap();
    let table = SequencePitchTable {
        values: Box::new(std::array::from_fn(|i| {
            fixture["values"][i].as_u64().unwrap() as u16
        })),
    };
    for row in fixture["initial"].as_array().unwrap() {
        let n = |i: usize| row[i].as_u64().unwrap() as u8;
        assert_eq!(
            u64::from(table.pitch(n(0), n(1), n(2)).unwrap()),
            row[3].as_u64().unwrap(),
            "{row}"
        );
    }
    for row in fixture["fine"].as_array().unwrap() {
        let n = |i: usize| row[i].as_u64().unwrap() as u16;
        assert_eq!(
            u64::from(
                table
                    .fine_pitch(n(0), n(1), n(2) as u8, n(3) as u8)
                    .unwrap()
            ),
            row[4].as_u64().unwrap(),
            "{row}"
        );
    }
    assert_eq!(fixture["initial"].as_array().unwrap().len(), 8192);
    assert_eq!(fixture["fine"].as_array().unwrap().len(), 1440);
}
