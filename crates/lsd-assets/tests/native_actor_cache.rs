use lsd_assets::formats::Mom;
use lsd_core::{ActorAxis, BodyWorldFrame, FixedMatrix, PsxMathTables};
use serde_json::{Value, json};
#[test]
fn complete_original_movement_wrapper_uses_cached_world_and_signed_margin() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/actor-cache-movement.json")).unwrap();
    let hex = fixture["owned_model_hex"].as_str().unwrap();
    let bytes = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect::<Vec<_>>();
    let model = Mom::parse(&bytes).unwrap();
    let mut math = PsxMathTables {
        sin_cos: Box::new([0; 4096]),
        sqrt_window: Box::new([0; 512]),
    };
    for row in fixture["owned_trig"].as_array().unwrap() {
        math.sin_cos[row[0].as_u64().unwrap() as usize] = row[1].as_u64().unwrap() as u32;
    }
    for (number, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let input = &case["input"];
        let cached_angles =
            std::array::from_fn(|i| input["cached_angles"][i].as_i64().unwrap() as i16);
        let current_angles =
            std::array::from_fn(|i| input["current_angles"][i].as_i64().unwrap() as i16);
        let scale = std::array::from_fn(|i| input["scale"][i].as_i64().unwrap() as i32);
        let world = BodyWorldFrame {
            matrix: math.rotation(cached_angles).scaled(scale),
            position: std::array::from_fn(|i| input["position"][i].as_i64().unwrap() as i32),
        };
        assert_eq!(
            json!(world.matrix.0.into_iter().flatten().collect::<Vec<_>>()),
            case["before"]["cached_matrix"]
        );
        let mut actor = model.construct_actor().unwrap();
        assert_eq!(actor.collision_margin, 0);
        actor.pose.position = world.position;
        actor.pose.angles = current_angles;
        actor.pose.scale = scale;
        actor.collision_margin = input["margin"].as_i64().unwrap() as i32;
        let geometry = model.collision_model(&actor).unwrap().unwrap();
        let axis = match input["axis"].as_str().unwrap() {
            "X" => ActorAxis::X,
            "Y" => ActorAxis::Y,
            "Z" => ActorAxis::Z,
            _ => unreachable!(),
        };
        actor.move_local(
            axis,
            input["amount"].as_i64().unwrap() as i32,
            math.rotation(current_angles),
        );
        let code = axis.notification();
        assert_eq!(
            json!(actor.pose.position),
            case["after_move"]["local_position"],
            "move {number}"
        );
        assert_eq!(
            json!(actor.move_distance),
            case["after_move"]["move_distance"]
        );
        assert_eq!(
            json!(actor.animation.actor_coordinate_cache),
            case["after_move"]["cache_flag"]
        );
        let prepared = geometry
            .prepared_actor_cloud(world, code, actor.move_distance, actor.collision_margin)
            .unwrap();
        assert_eq!(
            json!(prepared.world_position),
            case["notification"]["cached_position"],
            "cache {number}"
        );
        assert_eq!(
            json!(prepared.cloud.0.into_iter().flatten().collect::<Vec<_>>()),
            case["notification"]["cloud"],
            "cloud {number}: {input}"
        );
        assert_eq!(json!(code), case["notification"]["code"]);
        // GsGetLw alone reads an existing local matrix; render rebuild is a separate phase.
        assert_eq!(
            case["after_sdk_refresh"]["cached_position"],
            json!(actor.pose.position)
        );
        assert_eq!(
            case["after_sdk_refresh"]["cached_matrix"],
            case["before"]["cached_matrix"]
        );
        let refreshed = math.rotation(current_angles).scaled(scale);
        assert_eq!(
            json!(refreshed.0.into_iter().flatten().collect::<Vec<_>>()),
            case["after_matrix_refresh"]["cached_matrix"]
        );
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 270);
    let absent = lsd_core::CollisionModel {
        bounds: None,
        faces: vec![],
    };
    assert!(
        absent
            .prepared_actor_cloud(
                BodyWorldFrame {
                    matrix: FixedMatrix::IDENTITY,
                    position: [0; 3]
                },
                6,
                128,
                0
            )
            .is_none()
    );
}
