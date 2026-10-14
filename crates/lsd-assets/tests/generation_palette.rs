use lsd_assets::formats::{TimBlock, Vram};
use lsd_core::{GenerationEffectTables, VramRectangle, generation_border_moves};

#[test]
fn overlapping_palette_copy_rotates_each_strip_and_preserves_other_words() {
    let words: Vec<u16> = (0..1024 * 512)
        .map(|i| ((i * 173 + 29) & 65535) as u16)
        .collect();
    let bytes: Vec<u8> = words.iter().flat_map(|v| v.to_le_bytes()).collect();
    let mut vram = Vram::default();
    vram.upload_block(&TimBlock {
        x: 0,
        y: 0,
        width_words: 1024,
        height: 512,
        declared_bytes: 12 + bytes.len() as u32,
        payload: &bytes,
    })
    .unwrap();
    let rect = VramRectangle {
        x: 5,
        y: 100,
        width: 37,
        height: 6,
    };
    let scratch = VramRectangle {
        x: 101,
        y: 100,
        width: 1,
        height: 6,
    };
    let geometry = GenerationEffectTables {
        environment_origin: [0; 2],
        environment_size: [0; 2],
        gradient_origin: [0; 2],
        gradient_size: [0; 2],
        borders: [rect, scratch, rect, scratch],
    };
    let mut expected = words.clone();
    for tick in 1..=40 {
        for command in generation_border_moves(2, &geometry) {
            vram.copy_rectangle(command).unwrap();
        }
        for y in 100..106 {
            for x in 0..37 {
                expected[y * 1024 + 5 + x] = words[y * 1024 + 5 + (x + 37 - tick % 37) % 37];
            }
            expected[y * 1024 + 101] = words[y * 1024 + 5 + (37 - tick % 37) % 37];
        }
        assert_eq!(vram.words(), expected, "palette tick {tick}");
    }
    let before = vram.words().to_vec();
    for rect in [
        VramRectangle { x: -1, ..rect },
        VramRectangle { width: 0, ..rect },
        VramRectangle {
            width: i32::MAX,
            ..rect
        },
    ] {
        assert!(
            vram.copy_rectangle(lsd_core::VramMove {
                source: rect,
                destination: [0, 0]
            })
            .is_err()
        );
        assert_eq!(vram.words(), before);
    }
}
