use lsd_assets::formats::{
    Lbd, PixelMode, TextureSample, Tile, TimBlock, TimImage, TimList, Tix, Tmd, Vram,
};

fn block(x: u16, y: u16, width: u16, height: u16, words: &[u16]) -> Vec<u8> {
    let mut bytes = ((12 + words.len() * 2) as u32).to_le_bytes().to_vec();
    for value in [x, y, width, height] {
        bytes.extend(value.to_le_bytes());
    }
    for word in words {
        bytes.extend(word.to_le_bytes());
    }
    bytes
}

fn tim(flags: u32, palette: Option<Vec<u8>>, image: Vec<u8>) -> Vec<u8> {
    let mut bytes = 0x10u32.to_le_bytes().to_vec();
    bytes.extend(flags.to_le_bytes());
    if let Some(palette) = palette {
        bytes.extend(palette);
    }
    bytes.extend(image);
    bytes
}

#[test]
fn indexed_four_bit_pixels_use_low_nibble_first_and_the_selected_clut() {
    let palette: Vec<_> = (0..16).collect();
    let bytes = tim(
        8,
        Some(block(0, 480, 16, 1, &palette)),
        block(64, 0, 1, 1, &[0x3210]),
    );
    let image = TimImage::parse(&bytes).unwrap();
    assert_eq!(image.mode, PixelMode::Indexed4);
    assert_eq!(image.pixel_width(), 4);
    let mut vram = Vram::default();
    vram.upload(&image).unwrap();
    for x in 0..4 {
        assert_eq!(vram.sample(1, 480 << 6, [x, 0]).unwrap().word, u16::from(x));
    }
    assert_eq!(vram.sample(1, 480 << 6, [0, 0]).unwrap().rgba[3], 0);
}

#[test]
fn eight_bit_pixels_keep_all_palette_index_bits() {
    let palette: Vec<_> = (0..256).collect();
    let bytes = tim(
        9,
        Some(block(16, 480, 256, 1, &palette)),
        block(128, 0, 1, 1, &[0xFE02]),
    );
    let mut vram = Vram::default();
    vram.upload(&TimImage::parse(&bytes).unwrap()).unwrap();
    assert_eq!(vram.sample(0x82, (480 << 6) | 1, [0, 0]).unwrap().word, 2);
    assert_eq!(vram.sample(0x82, (480 << 6) | 1, [1, 0]).unwrap().word, 254);
}

#[test]
fn direct_pixels_preserve_transparency_stp_and_row_stride() {
    let bytes = tim(2, None, block(128, 256, 2, 2, &[0, 0x8000, 0x7FFF, 0xFC00]));
    let mut vram = Vram::default();
    vram.upload(&TimImage::parse(&bytes).unwrap()).unwrap();
    assert_eq!(vram.sample(0x112, 0, [0, 0]).unwrap().rgba, [0, 0, 0, 0]);
    let black = vram.sample(0x112, 0, [1, 0]).unwrap();
    assert_eq!(black.rgba, [0, 0, 0, 255]);
    assert!(black.semi_transparent);
    assert_eq!(
        vram.sample(0x112, 0, [0, 1]).unwrap().rgba,
        [255, 255, 255, 255]
    );
    assert_eq!(vram.sample(0x112, 0, [1, 1]).unwrap().word, 0xFC00);
}

#[test]
fn twenty_four_bit_images_are_not_misclassified_as_texture_pages() {
    let bytes = tim(3, None, block(0, 0, 3, 1, &[0x0201, 0x0403, 0x0605]));
    let image = TimImage::parse(&bytes).unwrap();
    assert_eq!(image.mode, PixelMode::Direct24);
    assert_eq!(image.pixel_width(), 2);
    assert_eq!(image.image.payload, [1, 2, 3, 4, 5, 6]);
    assert!(Vram::default().sample(0x180, 0, [0, 0]).is_err());
}

#[test]
fn native_image_rectangle_can_exceed_its_ignored_length_word() {
    let mut bytes = tim(2, None, block(0, 0, 2, 1, &[1, 2]));
    bytes[8..12].copy_from_slice(&12u32.to_le_bytes());
    let image = TimImage::parse(&bytes).unwrap();
    assert_eq!(image.image.declared_bytes, 12);
    assert_eq!(image.used_bytes, 24);
    assert_eq!(image.image.payload, [1, 0, 2, 0]);
    assert!(TimImage::parse(&bytes[..22]).is_err());
}

#[test]
fn truncated_palettes_and_reserved_tim_flags_are_rejected() {
    let mut bytes = tim(
        8,
        Some(block(0, 480, 16, 1, &[0; 16])),
        block(0, 0, 1, 1, &[0]),
    );
    bytes[8..12].copy_from_slice(&12u32.to_le_bytes());
    assert!(TimImage::parse(&bytes).is_err());
    assert!(TimImage::parse(&tim(4, None, block(0, 0, 1, 1, &[0]))).is_err());
}

#[test]
fn aliases_in_tim_lists_preserve_upload_order_and_entry_count() {
    let image = tim(2, None, block(0, 0, 1, 1, &[0x1234]));
    let mut bytes = Vec::new();
    for word in [2u32, 12, 12] {
        bytes.extend(word.to_le_bytes());
    }
    bytes.extend(image);
    let list = TimList::parse(&bytes).unwrap();
    assert_eq!(list.images.len(), 2);
    assert_eq!(list.offsets, [12, 12]);
    assert_eq!(list.images[0].image.payload, list.images[1].image.payload);
    bytes[4..8].copy_from_slice(&8u32.to_le_bytes());
    assert!(TimList::parse(&bytes).is_err());
}

#[test]
fn tix_banks_are_bounded_and_the_count_is_not_a_magic_signature() {
    let mut bank = 1u32.to_le_bytes().to_vec();
    bank.extend(8u32.to_le_bytes());
    bank.extend(tim(2, None, block(0, 0, 1, 1, &[0x1234])));
    let mut bytes = vec![0; 36];
    bytes[..4].copy_from_slice(&1u32.to_le_bytes());
    bytes[4..8].copy_from_slice(&36u32.to_le_bytes());
    bytes[20..24].copy_from_slice(&(bank.len() as u32).to_le_bytes());
    bytes.extend(bank);
    assert_eq!(Tix::parse(&bytes).unwrap().banks[0].images.len(), 1);
    bytes[..4].copy_from_slice(&5u32.to_le_bytes());
    assert!(Tix::parse(&bytes).is_err());
    bytes[..4].copy_from_slice(&1u32.to_le_bytes());
    bytes[4..8].copy_from_slice(&32u32.to_le_bytes());
    assert!(Tix::parse(&bytes).is_err());
}

#[test]
fn invalid_uploads_do_not_partially_modify_vram() {
    let mut vram = Vram::default();
    let before = vram.words().to_vec();
    let mut block = TimBlock {
        x: 0,
        y: 0,
        width_words: 2,
        height: 1,
        declared_bytes: 16,
        payload: &[1, 0],
    };
    assert!(vram.upload_block(&block).is_err());
    assert_eq!(vram.words(), before);
    block.x = 1023;
    block.payload = &[1, 0, 2, 0];
    assert!(vram.upload_block(&block).is_err());
    assert_eq!(vram.words(), before);
}

#[test]
fn color_expansion_keeps_opaque_black_separate_from_transparency() {
    assert_eq!(TextureSample::from_word(0).rgba[3], 0);
    assert_eq!(TextureSample::from_word(0x8000).rgba, [0, 0, 0, 255]);
    assert_eq!(TextureSample::from_word(0x001F).rgba, [255, 0, 0, 255]);
    assert_eq!(TextureSample::from_word(0x03E0).rgba, [0, 255, 0, 255]);
    assert_eq!(TextureSample::from_word(0x7C00).rgba, [0, 0, 255, 255]);
}

fn model(mode: u8, flag: u8, words: &[u32]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for word in [0x41u32, 0, 1, 28, 4, 60, 4, 92, 1, 0] {
        bytes.extend(word.to_le_bytes());
    }
    for i in 0..4i16 {
        for value in [i * 8, -i * 8, i * 16, 0] {
            bytes.extend(value.to_le_bytes());
        }
    }
    for _ in 0..4 {
        for value in [0i16, -4096, 0, 0] {
            bytes.extend(value.to_le_bytes());
        }
    }
    bytes.extend([9, words.len() as u8, flag, mode]);
    for word in words {
        bytes.extend(word.to_le_bytes());
    }
    bytes
}

#[test]
fn textured_unlit_gouraud_triangle_preserves_uv_colors_and_indices() {
    let words = [
        0x1234_0201,
        0x0082_0403,
        0x0000_0605,
        0x350C_0B0A,
        0x000F_0E0D,
        0x0012_1110,
        0x0001_0000,
        0x0000_0002,
    ];
    let tmd = Tmd::parse(&model(0x35, 1, &words)).unwrap();
    let polygon = &tmd.objects[0].polygons[0];
    assert_eq!(polygon.vertices, [0, 1, 2]);
    assert!(polygon.normals.is_empty());
    assert_eq!(polygon.uv, [[1, 2], [3, 4], [5, 6]]);
    assert_eq!(polygon.clut, Some(0x1234));
    assert_eq!(polygon.texture_page, Some(0x82));
    assert_eq!(
        polygon.colors,
        [[10, 11, 12, 0x35], [13, 14, 15, 0], [16, 17, 18, 0]]
    );
    assert!(polygon.gouraud());
    assert!(!polygon.lighting_enabled());
}

#[test]
fn lit_gouraud_pairs_keep_normal_and_vertex_indices_distinct() {
    let words = [0x307F_3F1F, 0x0002_0000, 0x0000_0001, 0x0001_0003];
    let tmd = Tmd::parse(&model(0x30, 0, &words)).unwrap();
    let object = &tmd.objects[0];
    let polygon = &object.polygons[0];
    assert_eq!(polygon.vertices, [2, 0, 1]);
    assert_eq!(polygon.normals, [0, 1, 3]);
    assert_eq!(object.normals[3].xyz, [0, -4096, 0]);
    assert_eq!(object.vertices[2].xyz, [16, -16, 32]);
    assert_eq!(polygon.colors.len(), 1);
}

#[test]
fn flat_textured_lit_quad_has_one_normal_and_four_vertices() {
    let words = [
        0x1234_0201,
        0x0082_0403,
        0x0000_0605,
        0x0000_0807,
        0x0002_0001,
        0x0000_0003,
        0x0000_0001,
    ];
    let tmd = Tmd::parse(&model(0x2E, 2, &words)).unwrap();
    let polygon = &tmd.objects[0].polygons[0];
    assert_eq!(polygon.vertices, [2, 3, 0, 1]);
    assert_eq!(polygon.normals, [1]);
    assert!(polygon.colors.is_empty());
    assert!(polygon.double_sided());
    assert!(polygon.semi_transparent());
}

#[test]
fn polygon_lengths_references_and_relocated_pointers_fail_explicitly() {
    let valid = model(0x21, 1, &[0x21FF_FFFF, 0x0001_0000, 0x0000_0002]);
    let mut bad = valid.clone();
    bad[104 + 1] = 2;
    assert!(Tmd::parse(&bad).is_err());
    let mut bad = valid.clone();
    bad[112..116].copy_from_slice(&0x0001_FFFFu32.to_le_bytes());
    assert!(Tmd::parse(&bad).is_err());
    let mut bad = valid.clone();
    bad[4..8].copy_from_slice(&1u32.to_le_bytes());
    assert!(Tmd::parse(&bad).is_err());
    assert!(Tmd::parse(&valid[..valid.len() - 1]).is_err());
}

fn empty_map(extra: usize) -> Vec<u8> {
    let grid = 24usize;
    let relative = 4808 + extra * 12;
    let mut bytes = vec![0; grid + relative + 12];
    bytes[4..8].copy_from_slice(&(grid as u32).to_le_bytes());
    bytes[8..12].copy_from_slice(&(relative as u32).to_le_bytes());
    bytes[grid + 4..grid + 8].copy_from_slice(&[20, 0, 20, 0]);
    bytes[grid + relative..grid + relative + 4].copy_from_slice(&0x41u32.to_le_bytes());
    bytes
}

#[test]
fn absent_tiles_do_not_dereference_unused_model_indices() {
    let mut bytes = empty_map(0);
    bytes[34..36].copy_from_slice(&0xFFFFu16.to_le_bytes());
    let map = Lbd::parse(&bytes).unwrap();
    assert_eq!(map.tiles.len(), 400);
    assert_eq!(map.dimensions, [20, 20]);
    assert!(!map.tiles[0].present());
    assert_eq!(map.tiles[0].model, 0xFFFF);
    bytes[32] = 1;
    assert!(Lbd::parse(&bytes).is_err());
}

#[test]
fn tile_chains_follow_grid_relative_offsets_and_reject_cycles() {
    let mut bytes = empty_map(1);
    bytes[40..44].copy_from_slice(&4808u32.to_le_bytes());
    let map = Lbd::parse(&bytes).unwrap();
    assert_eq!(map.cell_chain(0).unwrap().len(), 2);
    assert!(map.cell_chain(400).is_err());
    bytes[24 + 4808 + 8..24 + 4808 + 12].copy_from_slice(&8u32.to_le_bytes());
    assert!(Lbd::parse(&bytes).is_err());
    bytes[40..44].copy_from_slice(&9u32.to_le_bytes());
    assert!(Lbd::parse(&bytes).is_err());
}

#[test]
fn native_tile_pose_keeps_signed_height_and_heading_wrap() {
    let tile = Tile {
        tag: 1,
        category: 7,
        model: 3,
        flags: 9,
        quarter_turn: 255,
        height: -3,
        next_offset: 0,
    };
    assert_eq!(tile.native_position(21), [3072, -6144, 3072]);
    assert_eq!(tile.native_heading(), -1024);
}

#[test]
fn short_prefixes_return_errors_instead_of_panicking() {
    let bytes = empty_map(0);
    for length in 0..bytes.len() {
        assert!(Lbd::parse(&bytes[..length]).is_err());
    }
    let bytes = tim(2, None, block(0, 0, 2, 1, &[1, 2]));
    for length in 0..bytes.len() {
        assert!(TimImage::parse(&bytes[..length]).is_err());
    }
}
