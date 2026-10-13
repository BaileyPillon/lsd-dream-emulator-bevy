use lsd_core::{CollisionBounds, CollisionCloud, CollisionFace, PsxMathTables};
use serde_json::Value;

fn point(value: &Value) -> [i16; 3] {
    std::array::from_fn(|i| value[i].as_i64().unwrap() as i16)
}

#[test]
fn supplied_clouds_match_original_aggregator_with_semantic_gte() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/collision-cloud.json")).unwrap();
    let mut tables = PsxMathTables {
        sin_cos: Box::new([0; 4096]),
        sqrt_window: Box::new([0; 512]),
    };
    for (i, value) in tables.sqrt_window[320..].iter_mut().enumerate() {
        *value = (((i + 64) as f64).sqrt() * 512.0) as i16;
    }
    for (index, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let bounds = CollisionBounds {
            minimum: point(&case["bounds"][0]),
            maximum: point(&case["bounds"][1]),
        };
        let cloud = CollisionCloud(std::array::from_fn(|i| point(&case["cloud"][i])));
        let faces: Vec<_> = case["faces"]
            .as_array()
            .unwrap()
            .iter()
            .map(|face| {
                let points: Vec<_> = face.as_array().unwrap().iter().map(point).collect();
                match points.len() {
                    3 => CollisionFace::Triangle(points.try_into().unwrap()),
                    4 => CollisionFace::Quad(points.try_into().unwrap()),
                    _ => panic!("bad fixture"),
                }
            })
            .collect();
        let hit = cloud
            .intersection(
                bounds,
                &faces,
                case["filter_low"].as_bool().unwrap(),
                &tables,
            )
            .unwrap();
        assert_eq!(
            hit.is_some(),
            case["returned"].as_u64().unwrap() != 0,
            "case {index}"
        );
        if let Some(hit) = hit {
            assert_eq!(hit.position, point(&case["position"]), "case {index}");
        }
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 480);
}
