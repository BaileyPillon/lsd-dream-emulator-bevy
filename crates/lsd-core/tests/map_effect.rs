use lsd_core::{MapCompletionCommand, MapEffect, MapLinkCommand, MapLinkPolicy, PlayerControls};
use serde_json::Value;

#[test]
fn attached_effect_consumer_completes_at_the_original_counter_boundary() {
    let fixture: Value =
        serde_json::from_str(include_str!("fixtures/effect-lifecycle.json")).unwrap();
    let colors = |name: &str| {
        std::array::from_fn(|i| {
            std::array::from_fn(|j| fixture["palette"][name][i][j].as_u64().unwrap() as u8)
        })
    };
    let palette = lsd_core::EffectPalette {
        start: colors("start"),
        attached: colors("attached"),
    };
    for map_state in [4, 5, 6, 7, 8, 10] {
        for case in fixture["sequences"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|case| {
                case["attached"].as_bool().unwrap()
                    && case["initial"]["linked"] == 0
                    && case["variant"] == 0
            })
        {
            let mut consumer = MapEffect::new(palette.clone());
            let mut policy = MapLinkPolicy {
                state: map_state,
                tick: 0,
                location: 0,
                move_callback: 1,
                reset_view_pending: true,
                view_active: false,
            };
            let mut controls = PlayerControls {
                paused: 2,
                input_locked: true,
                ..PlayerControls::default()
            };
            let commands = consumer
                .consume(MapLinkCommand::Fade {
                    variant: 0,
                    mode: case["mode"].as_i64().unwrap() as i32,
                    color_step: case["initial"]["color_step"].as_i64().unwrap() as i32,
                    attach: true,
                })
                .unwrap();
            assert!(commands.is_empty());
            let outputs = case["observations"].as_array().unwrap();
            for expected in &outputs[1..outputs.len() - 1] {
                assert!(
                    consumer
                        .tick(&mut policy, &mut controls)
                        .unwrap()
                        .is_empty()
                );
                assert_eq!(
                    consumer.effect.life,
                    expected["state"]["life"].as_i64().unwrap() as i32
                );
                assert_eq!(
                    consumer.effect.color,
                    std::array::from_fn(|i| expected["state"]["color"][i].as_u64().unwrap() as u8)
                );
                assert!(consumer.map_attached && consumer.render_attached);
            }
            let commands = consumer.tick(&mut policy, &mut controls).unwrap();
            assert_eq!(commands[0], MapCompletionCommand::DetachEffect);
            assert_eq!(
                commands[1],
                MapCompletionCommand::SetStageColor(palette.attached[0])
            );
            assert_eq!(
                commands.last(),
                Some(&MapCompletionCommand::Notify(if map_state == 10 {
                    4
                } else {
                    map_state
                }))
            );
            assert!(!consumer.map_attached && !consumer.render_attached);
            assert_eq!(controls.paused, if map_state == 10 { 0 } else { 2 });
            assert!(controls.input_locked);
            let life = consumer.effect.life;
            assert!(
                consumer
                    .tick(&mut policy, &mut controls)
                    .unwrap()
                    .is_empty()
            );
            assert_eq!(consumer.effect.life, life);
        }
    }
}

#[test]
fn zero_step_preserves_the_existing_step_and_busy_start_keeps_the_life() {
    let palette = lsd_core::EffectPalette {
        start: [[0; 3]; 8],
        attached: [[0; 3]; 8],
    };
    let mut consumer = MapEffect::new(palette);
    consumer.effect.color_step = 5;
    consumer
        .consume(MapLinkCommand::Fade {
            variant: 4,
            mode: 0,
            color_step: 0,
            attach: true,
        })
        .unwrap();
    assert_eq!(consumer.effect.life, 51);
    consumer
        .consume(MapLinkCommand::Fade {
            variant: 7,
            mode: 9,
            color_step: 30,
            attach: false,
        })
        .unwrap();
    assert_eq!(consumer.effect.life, 51);
    assert_eq!(consumer.effect.channels, 4);
    assert_eq!(consumer.effect.color_step, 30);
    assert_eq!(consumer.effect.mode, 0);
}
