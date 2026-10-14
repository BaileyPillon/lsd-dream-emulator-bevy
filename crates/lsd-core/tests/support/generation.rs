use lsd_core::{GenerationActorEntry, GenerationEntry, GenerationTables};
use serde_json::Value;
pub fn num(v: &Value) -> i32 {
    v.as_i64().unwrap() as i32
}
pub fn arr<T: Copy, const N: usize>(v: &Value, read: impl Fn(&Value) -> T) -> [T; N] {
    std::array::from_fn(|i| read(&v[i]))
}
pub fn actor_entry(v: &Value) -> GenerationActorEntry {
    GenerationActorEntry {
        key: arr(&v["key"], |v| num(v) as i8),
        style: num(&v["style"]) as u8,
        effect: num(&v["effect"]) as i8,
        reserved: arr(&v["reserved"], |v| num(v) as u8),
    }
}
pub fn tables(v: &Value) -> GenerationTables {
    GenerationTables {
        entries: std::array::from_fn(|i| {
            v["entries"][i]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| GenerationEntry::from(arr(v, |v| num(v) as i8)))
                .collect()
        }),
        cycle: arr(&v["cycle"], |v| num(v) as i8),
        fixed: std::array::from_fn(|i| {
            if v["fixed"][i].is_null() {
                None
            } else {
                Some(GenerationEntry::from(arr(&v["fixed"][i], |v| num(v) as i8)))
            }
        }),
        palette: v["palette"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| arr(v, |v| num(v) as u8))
            .collect(),
        heights: arr(&v["heights"], num),
        sample_offsets: arr(&v["sample_offsets"], num),
        rotations: arr(&v["rotations"], |v| arr(v, |v| num(v) as u32)),
        scales: arr(&v["scales"], |v| arr(v, |v| num(v) as u32)),
        fixed_scale: arr(&v["fixed_scale"], |v| num(v) as u32),
        colors_a: arr(&v["colors_a"], |v| arr(v, |v| num(v) as u8)),
        colors_b: arr(&v["colors_b"], |v| arr(v, |v| num(v) as u8)),
        effect_palettes: if v["effect_palettes"].is_null() {
            [[[0; 3]; 18]; 2]
        } else {
            arr(&v["effect_palettes"], |v| {
                arr(v, |v| arr(v, |v| num(v) as u8))
            })
        },
        entity_ranges: arr(&v["entity_ranges"], num),
        actor_entries: std::array::from_fn(|i| {
            v["actor_entries"][i]
                .as_array()
                .unwrap()
                .iter()
                .map(actor_entry)
                .collect()
        }),
        styles: v["styles"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| arr(v, |v| num(v) as i16))
            .collect(),
        alternate_parameter: num(&v["alternate_parameter"]),
    }
}
