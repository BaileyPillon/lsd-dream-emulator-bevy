use lsd_core::{
    AnimationClip, AnimationError, AnimationFrame, AnimationParent, AnimationPlayer,
    AnimationRecord, AnimationWord,
};

fn record(part: u8, kind: u8, flags: u8, payload: &[u32]) -> AnimationRecord {
    AnimationRecord {
        header: AnimationWord {
            part_type: part,
            kind,
            flags,
            word_count: (payload.len() + 1) as u8,
        },
        payload: payload.to_vec(),
    }
}

fn clip(frames: Vec<Vec<AnimationRecord>>) -> AnimationClip {
    AnimationClip {
        tag: 0,
        header_halfword: 0,
        frames: frames
            .into_iter()
            .map(|records| AnimationFrame {
                tag: 0,
                metadata: 0,
                records,
            })
            .collect(),
    }
}

#[test]
fn selecting_and_first_tick_both_apply_frame_zero_and_single_frames_hold() {
    let clips = [
        clip(vec![
            vec![record(5, 1, 9, &[1, 0, 0])],
            vec![record(5, 1, 9, &[10, 0, 0])],
        ]),
        clip(vec![vec![record(5, 1, 9, &[100, 0, 0])]]),
    ];
    let mut player = AnimationPlayer::new(&[5], 0);
    player.select_clip(&clips, 0).unwrap();
    assert_eq!(player.parts[0].translation[0], 1);
    player.enabled = true;
    player.tick(&clips, |_| {}).unwrap();
    assert_eq!((player.parts[0].translation[0], player.frame_index), (2, 1));
    player.tick(&clips, |_| {}).unwrap();
    assert_eq!(
        (player.parts[0].translation[0], player.frame_index),
        (12, 0)
    );
    player.select_clip(&clips, 1).unwrap();
    player.tick(&clips, |_| {}).unwrap();
    assert_eq!(
        (player.parts[0].translation[0], player.frame_index),
        (112, 0)
    );
}

#[test]
fn action_can_pause_or_select_before_the_animation_runs_and_timer_wraps() {
    let clips = [clip(vec![vec![record(5, 1, 9, &[1, 0, 0])], vec![]])];
    let mut player = AnimationPlayer::new(&[5], 0);
    player.action_enabled = true;
    player.enabled = true;
    player.ticks = u32::MAX;
    player
        .tick(&clips, |actor| {
            assert_eq!(actor.ticks, 0);
            actor.select_clip(&clips, 0).unwrap();
        })
        .unwrap();
    assert_eq!(player.parts[0].translation[0], 2);
    player.tick(&clips, |actor| actor.enabled = false).unwrap();
    assert_eq!(player.frame_index, 1);
    assert_eq!(player.actor_coordinate_cache, 0);
}

#[test]
fn relative_angles_store_the_wrapped_halfword_before_signed_remainder() {
    let mut player = AnimationPlayer::new(&[5], 0);
    player.parts[0].angles = [32767, -32768, -1];
    player
        .apply_record(&record(5, 1, 3, &[360, (-360i32) as u32, (-359i32) as u32]))
        .unwrap();
    assert_eq!(player.parts[0].angles, [0, 4095, -1]);
}

#[test]
fn relative_scale_uses_low_product_word_and_position_wraps() {
    let mut player = AnimationPlayer::new(&[5], 0);
    player.parts[0].scale = [i32::MAX, i32::MIN, -4095];
    player.parts[0].translation = [i32::MAX, i32::MIN, 10];
    player
        .apply_record(&record(
            5,
            1,
            13,
            &[0xFFFF_0002, 4096, 1, u32::MAX, (-11i32) as u32],
        ))
        .unwrap();
    assert_eq!(player.parts[0].scale, [0, -524288, -4095]);
    assert_eq!(player.parts[0].translation, [i32::MIN, i32::MAX, -1]);
    assert_eq!(
        player.parts[0].coordinate_translation,
        player.parts[0].translation
    );
}

#[test]
fn first_parent_keeps_ownership_and_initial_parenting_clears_only_coordinate_position() {
    let mut player = AnimationPlayer::new(&[5, 6], 0);
    player.apply_record(&record(5, 1, 8, &[1, 2, 3])).unwrap();
    player.apply_record(&record(5, 3, 0, &[0xFFFF])).unwrap();
    player.apply_record(&record(6, 3, 0, &[0])).unwrap();
    assert_eq!(player.actor_children, [1, 0]);
    assert_eq!(player.parts[0].translation, [1, 2, 3]);
    assert_eq!(player.parts[0].coordinate_translation, [0; 3]);
    player.apply_record(&record(5, 1, 9, &[4, 5, 6])).unwrap();
    player.apply_record(&record(5, 3, 0, &[6])).unwrap();
    assert_eq!(player.parts[0].parent, Some(AnimationParent::Actor));
    assert_eq!(player.parts[0].coordinate_translation, [5, 7, 9]);
    assert!(player.parts[1].children.is_empty());
}

#[test]
fn primitive_selection_is_one_based_and_existing_bindings_ignore_later_selection() {
    let mut player = AnimationPlayer::new(&[5], 2);
    player.apply_record(&record(5, 2, 0, &[0])).unwrap();
    assert_eq!(player.parts[0].primitive, None);
    player
        .apply_record(&record(5, 2, 0, &[0x1234_0002]))
        .unwrap();
    player.apply_record(&record(5, 2, 0, &[65535])).unwrap();
    assert_eq!(player.parts[0].primitive, Some(1));
}

#[test]
fn duplicate_part_types_use_first_match_and_unknown_targets_skip_payload_reads() {
    let mut player = AnimationPlayer::new(&[5, 5], 0);
    player
        .apply_record(&record(5, 0, 0, &[u32::MAX, 7]))
        .unwrap();
    player.apply_record(&record(9, 1, 14, &[])).unwrap();
    assert_eq!(player.parts[0].attributes, 7);
    assert_eq!(player.parts[1].attributes, 0);
    assert_eq!(player.parts[1].coordinate_cache, 1);
    player.parts[0].coordinate_cache = 9;
    player.apply_record(&record(5, 15, 0, &[])).unwrap();
    assert_eq!(player.parts[0].coordinate_cache, 0);
}

#[test]
fn invalid_public_inputs_return_explicit_errors() {
    let mut player = AnimationPlayer::new(&[5], 1);
    assert_eq!(
        player.apply_record(&record(5, 1, 14, &[])),
        Err(AnimationError::InvalidRecord)
    );
    assert_eq!(
        player.apply_record(&record(5, 2, 0, &[2])),
        Err(AnimationError::MissingPrimitive)
    );
    assert_eq!(
        player.apply_record(&record(5, 3, 0, &[9])),
        Err(AnimationError::MissingParent)
    );
    assert_eq!(player.select_clip(&[], 0), Err(AnimationError::InvalidClip));
    assert_eq!(
        player.select_clip(&[clip(vec![])], 0),
        Err(AnimationError::EmptyClip)
    );
}
