use lsd_core::{
    GenerationChild, GenerationObjectTables, GenerationPose, GenerationSpawn, GenerationStructure,
};
use serde_json::{Value, json};
fn num(v: &Value) -> i32 {
    v.as_i64().unwrap() as i32
}
fn arr<T: Copy, const N: usize>(v: &Value, read: impl Fn(&Value) -> T) -> [T; N] {
    std::array::from_fn(|i| read(&v[i]))
}
pub fn tables(v: &Value) -> GenerationObjectTables {
    GenerationObjectTables {
        model_indices: arr(&v["model_indices"], |v| num(v) as u32),
        palette_coordinates: arr(&v["palette_coordinates"], |v| num(v) as i16),
        child_origin: arr(&v["child_origin"], num),
        child_spacing: arr(&v["child_spacing"], num),
        child_motion: arr(&v["child_motion"], num),
        motion_translation: arr(&v["motion_translation"], num),
        motion_rotation: arr(&v["motion_rotation"], |v| num(v) as u32),
        sprite_x_offsets: arr(&v["sprite_x_offsets"], num),
        sprite_scale_even: arr(&v["sprite_scale_even"], |v| num(v) as u32),
        sprite_scale_odd: arr(&v["sprite_scale_odd"], |v| num(v) as u32),
        sprite_scale_low: arr(&v["sprite_scale_low"], |v| num(v) as u32),
        sprite_translation: arr(&v["sprite_translation"], num),
        sprite_random_scales: arr(&v["sprite_random_scales"], |v| arr(v, |v| num(v) as u32)),
        sprite_rectangles: arr(&v["sprite_rectangles"], |v| arr(v, |v| num(v) as u16)),
        sprite_cluts: arr(&v["sprite_cluts"], |v| arr(v, |v| num(v) as i16)),
    }
}
pub fn spawn(v: &Value) -> GenerationSpawn {
    GenerationSpawn {
        position: arr(&v["position"], num),
        rotation: arr(&v["rotation"], |v| num(v) as u32),
        scale: arr(&v["scale"], |v| num(v) as u32),
        index: num(&v["index"]),
        random: num(&v["random"]),
        color: arr(&v["color"], |v| num(v) as u8),
        alternate_color: if v["alternate_color"].is_null() {
            None
        } else {
            Some(arr(&v["alternate_color"], |v| num(v) as u8))
        },
    }
}
fn pose(v: GenerationPose) -> Value {
    json!({"position":v.position,"scale":v.scale,"angles":v.angles})
}
pub fn snapshot(v: &GenerationStructure) -> Value {
    let children:Vec<_>=v.children.iter().map(|child|match child {
        GenerationChild::Model{pose:p,model_index}=>json!({"kind":"model","pose":pose(*p),"model_index":model_index}),
        GenerationChild::Sprite(sprite)=>json!({"kind":"sprite","pose":pose(sprite.pose),"sprite_id":sprite.sprite_id,
            "color":sprite.color,"flags":sprite.flags,"scale":sprite.scale,"rotation":sprite.rotation,
            "rectangle":sprite.rectangle,"clut":sprite.clut}),
    }).collect();
    json!({"mode":v.mode,"pose":pose(v.pose),"model_index":v.model_index,"children":children,"tick":v.tick})
}
