use lsd_core::{GenerationEffectTables, GenerationLayer, GenerationLayers, VramRectangle};
use serde_json::{Value, json};
fn num(v: &Value) -> i32 {
    v.as_i64().unwrap() as i32
}
pub fn arr<T: Copy, const N: usize>(v: &Value, read: impl Fn(&Value) -> T) -> [T; N] {
    std::array::from_fn(|i| read(&v[i]))
}
pub fn geometry(v: &Value) -> GenerationEffectTables {
    GenerationEffectTables {
        environment_origin: arr(&v["environment_origin"], num),
        environment_size: arr(&v["environment_size"], num),
        gradient_origin: arr(&v["gradient_origin"], num),
        gradient_size: arr(&v["gradient_size"], num),
        borders: arr(&v["borders"], |v| VramRectangle {
            x: num(&v["x"]) as i16,
            y: num(&v["y"]) as i16,
            width: num(&v["width"]),
            height: num(&v["height"]),
        }),
    }
}
pub fn layer(v: GenerationLayer) -> Value {
    json!({"offset":v.offset,"size":v.size,"color":v.color,"flags":v.flags,"depth":v.depth,"parent":v.parent,"relative":v.relative})
}
pub fn snapshot(v: &GenerationLayers) -> Value {
    json!({"environment":v.environment.map(layer),"gradient":v.gradient.iter().copied().map(layer).collect::<Vec<_>>(),"stage_color":v.stage_color.unwrap_or([21,43,65])})
}
