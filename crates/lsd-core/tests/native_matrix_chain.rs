use lsd_core::FixedMatrix;
use serde_json::Value;
fn matrix(value: &Value) -> FixedMatrix {
    FixedMatrix(std::array::from_fn(|row| {
        std::array::from_fn(|column| value[row][column].as_i64().unwrap() as i16)
    }))
}
#[test]
fn sdk_short_matrix_application_and_multiplication_match_original_cpu_with_semantic_gte() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/matrix-chain.json")).unwrap();
    for (index, case) in fixture["applications"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        let vector = std::array::from_fn(|axis| case["vector"][axis].as_i64().unwrap() as i16);
        assert_eq!(
            matrix(&case["matrix"]).apply_saturated(vector),
            std::array::from_fn(|axis| case["result"][axis].as_i64().unwrap() as i16),
            "apply {index}"
        );
    }
    for (index, case) in fixture["multiplications"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
    {
        assert_eq!(
            matrix(&case["left"]).multiply(matrix(&case["right"])),
            matrix(&case["result"]),
            "multiply {index}"
        );
    }
}
