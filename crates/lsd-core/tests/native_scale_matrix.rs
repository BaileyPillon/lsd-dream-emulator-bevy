use lsd_core::FixedMatrix;
use serde_json::Value;
#[test]
fn sdk_scale_matrix_matches_signed_columns_and_overflow() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/scale-matrix.json")).unwrap();
    for case in fixture["cases"].as_array().unwrap() {
        let matrix = FixedMatrix(std::array::from_fn(|r| {
            std::array::from_fn(|c| case["matrix"][r][c].as_i64().unwrap() as i16)
        }));
        let scale = std::array::from_fn(|i| case["scale"][i].as_i64().unwrap() as i32);
        assert_eq!(
            serde_json::to_value(matrix.scaled(scale).0).unwrap(),
            case["expected"]
        );
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 10);
}
