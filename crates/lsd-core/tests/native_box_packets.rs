#[path = "support/generation.rs"]
mod generation_support;
#[path = "support/generation_layers.rs"]
#[allow(dead_code)]
mod layer_support;
use lsd_core::{BoxFramebuffer, GenerationLayer, GenerationLayers};
use serde_json::{Value, json};

#[test]
fn original_recursive_box_sort_sdk_packets_and_order_match() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/box-packets.json")).unwrap();
    let baseline: Value = serde_json::from_str(include_str!("fixtures/generation.json")).unwrap();
    let mut tables = generation_support::tables(&baseline["tables"]);
    tables.effect_palettes = layer_support::arr(&fixture["palettes"], |v| {
        layer_support::arr(v, |v| layer_support::arr(v, |v| v.as_u64().unwrap() as u8))
    });
    let geometry = layer_support::geometry(&fixture["geometry"]);
    let mut layers = GenerationLayers::default();
    for (index, case) in fixture["cases"].as_array().unwrap().iter().enumerate() {
        let flags = case["flags"].as_i64().unwrap() as i32;
        let palette = case["palette"].as_u64().unwrap() as usize;
        if index % 4 == 0 {
            layers = GenerationLayers::default();
            layers.create(
                &tables,
                &geometry,
                case["environment"]
                    .as_bool()
                    .unwrap()
                    .then_some([33, 66, 99]),
                flags,
                palette,
            );
        }
        layers.update(
            &tables,
            &geometry,
            flags,
            palette,
            [249, 3, 254],
            [case["eye"].as_i64().unwrap() as i32, 0],
        );
        let screen = layer_support::arr(&case["screen"], |v| v.as_i64().unwrap() as i32);
        let background: Vec<_> = layers
            .gradient
            .iter()
            .filter_map(|layer| layer.gpu_box_packet(screen))
            .collect();
        let foreground: Vec<_> = layers
            .environment
            .and_then(|layer| layer.gpu_box_packet(screen))
            .into_iter()
            .collect();
        assert_eq!(
            json!(
                background
                    .iter()
                    .map(|packet| packet.words())
                    .collect::<Vec<_>>()
            ),
            case["background_packets"],
            "background case {index}"
        );
        assert_eq!(
            json!(
                foreground
                    .iter()
                    .map(|packet| packet.words())
                    .collect::<Vec<_>>()
            ),
            case["foreground_packets"],
            "foreground case {index}"
        );
        let mut framebuffer = BoxFramebuffer::new(screen[0] as u16, screen[1] as u16, [21, 43, 65]);
        for packet in background.into_iter().chain(foreground) {
            framebuffer.draw_box(packet);
        }
        let expected: Vec<u16> = case["expected_rows"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|row| {
                row.as_array().unwrap().iter().flat_map(|run| {
                    std::iter::repeat_n(
                        run[1].as_u64().unwrap() as u16,
                        run[0].as_u64().unwrap() as usize,
                    )
                })
            })
            .collect();
        assert_eq!(
            framebuffer.pixels(),
            expected,
            "documented RGB555 model case {index}"
        );
    }
    for case in fixture["single_boxes"].as_array().unwrap() {
        let mut layer = GenerationLayer {
            offset: layer_support::arr(&case["offset"], |v| v.as_i64().unwrap() as i32),
            size: layer_support::arr(&case["size"], |v| v.as_u64().unwrap() as u16),
            color: layer_support::arr(&case["color"], |v| v.as_u64().unwrap() as u8),
            flags: case["flags"].as_u64().unwrap() as u32,
            depth: 8191,
            parent: None,
            relative: case["relative"].as_bool().unwrap(),
        };
        assert_eq!(
            json!(layer.gpu_box_packet([320, 240]).unwrap().words()),
            case["expected"]
        );
        layer.flags |= 1 << 31;
        assert!(layer.gpu_box_packet([320, 240]).is_none());
    }
}
