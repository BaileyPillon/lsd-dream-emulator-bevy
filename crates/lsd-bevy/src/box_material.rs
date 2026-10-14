//! Integer screen-box composition over the development scene image.
use bevy::{
    asset::embedded_asset,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};
use lsd_core::GpuBoxPacket;

pub const MAX_BOXES: usize = 128;
#[derive(Debug, Clone, ShaderType)]
pub struct BoxUniform {
    pub counts: UVec4,
    pub rectangles: [IVec4; MAX_BOXES],
    pub colors: [UVec4; MAX_BOXES],
}
impl Default for BoxUniform {
    fn default() -> Self {
        Self {
            counts: UVec4::new(0, 320, 240, 1),
            rectangles: [IVec4::ZERO; MAX_BOXES],
            colors: [UVec4::ZERO; MAX_BOXES],
        }
    }
}
impl BoxUniform {
    pub fn set_packets(&mut self, packets: &[GpuBoxPacket]) {
        assert!(
            packets.len() <= MAX_BOXES,
            "screen-box uniform capacity exceeded"
        );
        self.counts.x = packets.len() as u32;
        for (i, packet) in packets.iter().enumerate() {
            self.rectangles[i] = IVec4::from_array(packet.rectangle());
            let front = packet.foreground();
            self.colors[i] = UVec4::new(
                u32::from(front & 31),
                u32::from((front >> 5) & 31),
                u32::from((front >> 10) & 31),
                u32::from(packet.blend_mode()) | if packet.semi_transparent() { 4 } else { 0 },
            );
        }
    }
}
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct BoxMaterial {
    #[uniform(0)]
    pub boxes: BoxUniform,
    #[texture(1)]
    #[sampler(2)]
    pub scene: Handle<Image>,
}
impl Material for BoxMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://lsd_bevy/shaders/boxes.wesl".into()
    }
    fn enable_prepass() -> bool {
        false
    }
    fn enable_shadows() -> bool {
        false
    }
}
pub struct BoxMaterialPlugin;
impl Plugin for BoxMaterialPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shaders/boxes.wesl");
        app.add_plugins(MaterialPlugin::<BoxMaterial>::default());
    }
}
