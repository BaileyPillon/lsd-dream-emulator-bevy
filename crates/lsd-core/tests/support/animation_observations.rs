use lsd_core::{
    AnimationClip, AnimationFrame, AnimationParent, AnimationPlayer, AnimationRecord, AnimationWord,
};
use serde_json::{Value, json};

fn numbers<const N: usize, T>(value: &Value, convert: impl Fn(i64) -> T) -> [T; N] {
    std::array::from_fn(|i| convert(value[i].as_i64().unwrap()))
}

fn snapshot(player: &AnimationPlayer) -> Value {
    json!({"ticks": player.ticks, "enabled": player.enabled, "action_enabled": player.action_enabled,
        "clip": player.selected_clip, "frame": player.frame_index,
        "actor_cache": player.actor_coordinate_cache, "actor_children": player.actor_children,
        "parts": player.parts.iter().map(|part| json!({"part_type": part.part_type,
            "attributes": part.attributes, "cache": part.coordinate_cache, "angles": part.angles,
            "scale": part.scale, "position": part.translation, "coordinate_position": part.coordinate_translation,
            "primitive": part.primitive, "parent": part.parent.map(|parent| match parent {
                AnimationParent::Actor => -1i64, AnimationParent::Part(index) => index as i64 }),
            "children": part.children})).collect::<Vec<_>>()})
}

pub fn compare_cases(
    fixture: &Value,
    decoded_clips: Option<&[Vec<AnimationClip>]>,
) -> (usize, usize) {
    let mut observations = 0;
    let cases = fixture["cases"].as_array().unwrap();
    for (case_index, case) in cases.iter().enumerate() {
        let types: Vec<_> = case["types"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_u64().unwrap() as u8)
            .collect();
        let mut player =
            AnimationPlayer::new(&types, case["primitive_count"].as_u64().unwrap() as usize);
        let initial = &case["initial"];
        player.ticks = initial["ticks"].as_u64().unwrap() as u32;
        player.actor_coordinate_cache = initial["actor_cache"].as_i64().unwrap() as i32;
        for (part, state) in player
            .parts
            .iter_mut()
            .zip(initial["parts"].as_array().unwrap())
        {
            part.attributes = state["attributes"].as_u64().unwrap() as u32;
            part.coordinate_cache = state["cache"].as_i64().unwrap() as i32;
            part.angles = numbers(&state["angles"], |value| value as i16);
            part.scale = numbers(&state["scale"], |value| value as i32);
            part.translation = numbers(&state["position"], |value| value as i32);
            part.coordinate_translation =
                numbers(&state["coordinate_position"], |value| value as i32);
        }
        assert_eq!(snapshot(&player), *initial, "initial case {case_index}");
        let clips: Vec<_> = case["clips"]
            .as_array()
            .unwrap()
            .iter()
            .map(|frames| AnimationClip {
                tag: 0,
                header_halfword: 0,
                frames: frames
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|records| AnimationFrame {
                        tag: 0,
                        metadata: 0,
                        records: records
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|words| {
                                let words: Vec<_> = words
                                    .as_array()
                                    .unwrap()
                                    .iter()
                                    .map(|word| word.as_u64().unwrap() as u32)
                                    .collect();
                                AnimationRecord {
                                    header: AnimationWord::decode(words[0]),
                                    payload: words[1..].to_vec(),
                                }
                            })
                            .collect(),
                    })
                    .collect(),
            })
            .collect();
        let clips = decoded_clips.map_or(clips, |decoded| decoded[case_index].clone());
        for (step, operation) in case["operations"].as_array().unwrap().iter().enumerate() {
            match operation["op"].as_str().unwrap() {
                "select" => player
                    .select_clip(&clips, operation["index"].as_u64().unwrap() as usize)
                    .unwrap(),
                "enabled" => player.enabled = operation["value"].as_bool().unwrap(),
                "set_ticks" => {
                    player.ticks = operation["value"].as_u64().unwrap() as u32;
                    continue;
                }
                "tick" => player
                    .tick(&clips, |_| panic!("native fixture has actions disabled"))
                    .unwrap(),
                unknown => panic!("unknown native fixture operation {unknown}"),
            }
            assert_eq!(
                snapshot(&player),
                operation["state"],
                "case {case_index} operation {step}"
            );
            observations += 1;
        }
    }
    (cases.len(), observations)
}
