use lsd_assets::formats::{Lbd, Mml, Mom};

fn record(part: u8, kind: u8, flags: u8, payload: &[u32]) -> Vec<u32> {
    let mut words = vec![
        u32::from(part)
            | (u32::from(kind) << 16)
            | (u32::from(flags) << 20)
            | (((payload.len() + 1) as u32) << 24),
    ];
    words.extend(payload);
    words
}

fn clip(frames: &[Vec<Vec<u32>>]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(0x1234u16.to_le_bytes());
    bytes.extend(0x5678u16.to_le_bytes());
    bytes.extend((frames.len() as u32).to_le_bytes());
    for frame in frames {
        bytes.extend(0xABCDu16.to_le_bytes());
        bytes.extend((frame.len() as u16).to_le_bytes());
        bytes.extend(0xFEDC_BA98u32.to_le_bytes());
        for words in frame {
            for word in words {
                bytes.extend(word.to_le_bytes());
            }
        }
    }
    bytes
}

fn mom(clips: &[Vec<u8>], order: &[usize], objects: usize) -> Vec<u8> {
    let mut bytes = vec![0; 20 + order.len() * 4];
    bytes[..4].copy_from_slice(b"MOM ");
    bytes[12..16].copy_from_slice(b"MOS ");
    bytes[16..20].copy_from_slice(&(order.len() as u32).to_le_bytes());
    let mut starts = Vec::new();
    for clip in clips {
        starts.push(bytes.len());
        bytes.extend(clip);
    }
    for (i, &index) in order.iter().enumerate() {
        bytes[20 + i * 4..24 + i * 4].copy_from_slice(&((starts[index] - 12) as u32).to_le_bytes());
    }
    let tmd = bytes.len() as u32;
    bytes[8..12].copy_from_slice(&tmd.to_le_bytes());
    for word in [0x41, 0, objects as u32] {
        bytes.extend(word.to_le_bytes());
    }
    bytes.resize(bytes.len() + objects * 28, 0);
    let size = bytes.len() as u32;
    bytes[4..8].copy_from_slice(&size.to_le_bytes());
    bytes
}

fn mml(models: &[Vec<u8>], order: &[usize]) -> Vec<u8> {
    let mut bytes = vec![0; 8 + order.len() * 4];
    bytes[..4].copy_from_slice(b"MML ");
    bytes[4..8].copy_from_slice(&(order.len() as u32).to_le_bytes());
    let mut starts = Vec::new();
    for model in models {
        starts.push(bytes.len());
        bytes.extend(model);
    }
    for (i, &index) in order.iter().enumerate() {
        bytes[8 + i * 4..12 + i * 4].copy_from_slice(&(starts[index] as u32).to_le_bytes());
    }
    bytes
}

#[test]
fn mos_preserves_frame_headers_command_order_and_duplicate_part_types() {
    let frames = [vec![
        record(7, 8, 0, &[]),
        record(7, 8, 0, &[]),
        record(7, 1, 8, &[u32::MAX, 2, 3]),
        record(0, 15, 0, &[]),
    ]];
    let bytes = mom(&[clip(&frames)], &[0], 0);
    let model = Mom::parse(&bytes).unwrap();
    assert_eq!(model.initial_part_types, [7, 7]);
    let animation = &model.animations[0];
    assert_eq!((animation.tag, animation.header_halfword), (0x1234, 0x5678));
    assert_eq!(
        (animation.frames[0].tag, animation.frames[0].metadata),
        (0xABCD, 0xFEDC_BA98)
    );
    assert_eq!(animation.frames[0].records[2].payload, [u32::MAX, 2, 3]);
    assert_eq!(animation.frames[0].records[3].header.kind, 15);
}

#[test]
fn mos_aliases_and_unsorted_offsets_keep_selection_order() {
    let clips = [
        clip(&[vec![record(3, 8, 0, &[])]]),
        clip(&[vec![record(9, 8, 0, &[])]]),
    ];
    let bytes = mom(&clips, &[1, 0, 1], 0);
    let model = Mom::parse(&bytes).unwrap();
    assert_eq!(model.initial_part_types, [9]);
    assert!(model.animation_offsets[0] > model.animation_offsets[1]);
    assert_eq!(model.animation_offsets[0], model.animation_offsets[2]);
    assert_eq!(model.animations[0], model.animations[2]);
    assert_eq!(model.animations[1].frames[0].records[0].header.part_type, 3);
}

#[test]
fn mml_aliases_are_bounded_and_preserve_model_selection_order() {
    let first = mom(&[clip(&[vec![record(1, 8, 0, &[])]])], &[0], 0);
    let second = mom(&[clip(&[vec![record(2, 8, 0, &[])]])], &[0], 0);
    let mut bytes = mml(&[first, second], &[1, 0, 1]);
    let library = Mml::parse(&bytes).unwrap();
    assert_eq!(library.models.len(), 3);
    assert_eq!(library.models[0].initial_part_types, [2]);
    assert_eq!(library.models[1].initial_part_types, [1]);
    assert_eq!(library.offsets[0], library.offsets[2]);
    let first_at = library.offsets[1];
    bytes[first_at + 4..first_at + 8].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(Mml::parse(&bytes).is_err());
}

#[test]
fn animation_and_library_short_prefixes_return_errors() {
    let model = mom(&[clip(&[vec![record(1, 8, 0, &[])]])], &[0], 0);
    for end in 0..model.len() {
        assert!(Mom::parse(&model[..end]).is_err());
    }
    let library = mml(&[model], &[0]);
    for end in 0..library.len() {
        assert!(Mml::parse(&library[..end]).is_err());
    }
}

#[test]
fn zero_length_short_payloads_and_unexplained_animation_tails_are_rejected() {
    for words in [
        vec![0],
        record(1, 0, 0, &[0]),
        record(1, 1, 14, &[0; 7]),
        record(1, 2, 0, &[]),
        record(1, 3, 0, &[]),
    ] {
        let bytes = mom(&[clip(&[vec![words]])], &[0], 0);
        assert!(Mom::parse(&bytes).is_err());
    }
    let mut animation = clip(&[vec![record(1, 8, 0, &[])]]);
    animation.extend([0; 4]);
    assert!(Mom::parse(&mom(&[animation], &[0], 0)).is_err());
}

#[test]
fn invalid_table_offsets_and_impossible_frame_counts_are_rejected() {
    let valid = mom(&[clip(&[vec![record(1, 8, 0, &[])]])], &[0], 0);
    for (at, value) in [
        (8, 16),
        (8, u32::MAX),
        (16, u32::MAX),
        (20, 0),
        (20, 13),
        (20, u32::MAX),
        (28, u32::MAX),
    ] {
        let mut bad = valid.clone();
        bad[at..at + 4].copy_from_slice(&value.to_le_bytes());
        assert!(Mom::parse(&bad).is_err());
    }
    let valid = mml(&[valid], &[0]);
    for value in [0u32, 8, 13, u32::MAX] {
        let mut bad = valid.clone();
        bad[8..12].copy_from_slice(&value.to_le_bytes());
        assert!(Mml::parse(&bad).is_err());
    }
}

#[test]
fn active_bindings_are_checked_but_unknown_command_targets_follow_native_skip() {
    let declarations = record(1, 8, 0, &[]);
    let valid = clip(&[vec![
        declarations.clone(),
        record(1, 2, 0, &[0xABCD_0001]),
        record(1, 3, 0, &[0xFFFF]),
        record(1, 3, 0, &[0xABCD_0001]),
        record(9, 2, 0, &[65535]),
        record(9, 3, 0, &[99]),
    ]]);
    assert!(Mom::parse(&mom(&[valid], &[0], 1)).is_ok());
    for command in [record(1, 2, 0, &[2]), record(1, 3, 0, &[2])] {
        let invalid = clip(&[vec![declarations.clone(), command]]);
        assert!(Mom::parse(&mom(&[invalid], &[0], 1)).is_err());
    }
}

#[test]
fn lbd_decodes_the_bounded_streamed_library() {
    let model = mom(&[clip(&[vec![record(5, 8, 0, &[])]])], &[0], 0);
    let library = mml(&[model], &[0]);
    let at = 24 + 4808 + 12;
    let mut bytes = vec![0; at];
    bytes[2..4].copy_from_slice(&1u16.to_le_bytes());
    bytes[4..8].copy_from_slice(&24u32.to_le_bytes());
    bytes[8..12].copy_from_slice(&4808u32.to_le_bytes());
    bytes[16..20].copy_from_slice(&(at as u32).to_le_bytes());
    bytes[20..24].copy_from_slice(&(library.len() as u32).to_le_bytes());
    bytes[at - 12..at - 8].copy_from_slice(&0x41u32.to_le_bytes());
    bytes.extend(library);
    let map = Lbd::parse(&bytes).unwrap();
    assert_eq!(map.model_library_extent, Some(at..bytes.len()));
    assert_eq!(map.model_library.unwrap().models[0].initial_part_types, [5]);
    bytes[at + 8..at + 12].copy_from_slice(&8u32.to_le_bytes());
    assert!(Lbd::parse(&bytes).is_err());
}
