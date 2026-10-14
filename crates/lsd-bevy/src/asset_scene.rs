//! Mesh/material construction for asset inspection; original PS1 rendering remains open.
use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    mesh::Indices,
    prelude::*,
    render::render_resource::{Extent3d, Face, PrimitiveTopology, TextureDimension, TextureFormat},
};
use lsd_assets::formats::{Polygon, TmdObject, Vram};
use std::collections::BTreeMap;

pub type MeshBinding = (Handle<Mesh>, Handle<StandardMaterial>);

#[derive(Default)]
struct MeshData {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    colors: Vec<[f32; 4]>,
    uv: Vec<[f32; 2]>,
    indices: Vec<u32>,
}

type MaterialKey = (Option<u16>, Option<u16>, u8, u8);

fn rotate(vector: [i16; 3], quarter: u8) -> [f32; 3] {
    let [x, y, z] = vector.map(i32::from);
    let (x, z) = match quarter & 3 {
        0 => (x, z),
        1 => (z, -x),
        2 => (-x, -z),
        _ => (-z, x),
    };
    // PS1 +Y points down and +Z points forward; Bevy uses +Y up and camera -Z.
    [x as f32 / 2048.0, -y as f32 / 2048.0, -z as f32 / 2048.0]
}

fn append_polygon(data: &mut MeshData, object: &TmdObject, polygon: &Polygon, quarter: u8) {
    let base = data.positions.len() as u32;
    let positions: Vec<_> = polygon
        .vertices
        .iter()
        .map(|&index| rotate(object.vertices[usize::from(index)].xyz, quarter))
        .collect();
    let face_normal = (Vec3::from_array(positions[2]) - Vec3::from_array(positions[0]))
        .cross(Vec3::from_array(positions[1]) - Vec3::from_array(positions[0]))
        .normalize_or_zero()
        .to_array();
    for (i, position) in positions.iter().enumerate() {
        data.positions.push(*position);
        let normal = if polygon.normals.is_empty() {
            face_normal
        } else {
            let index = polygon.normals[if polygon.normals.len() == 1 { 0 } else { i }];
            Vec3::from_array(rotate(object.normals[usize::from(index)].xyz, quarter))
                .normalize_or_zero()
                .to_array()
        };
        data.normals.push(normal);
        let color = if polygon.colors.is_empty()
            || (polygon.texture_page.is_some() && polygon.texture_raw())
        {
            [1.0, 1.0, 1.0, 1.0]
        } else {
            let rgba = polygon.colors[if polygon.colors.len() == 1 { 0 } else { i }];
            Color::srgb_u8(rgba[0], rgba[1], rgba[2])
                .to_linear()
                .to_f32_array()
        };
        data.colors.push(color);
        data.uv.push(if polygon.uv.is_empty() {
            [0.0, 0.0]
        } else {
            [
                (f32::from(polygon.uv[i][0]) + 0.5) / 256.0,
                (f32::from(polygon.uv[i][1]) + 0.5) / 256.0,
            ]
        });
    }
    data.indices.extend([base, base + 2, base + 1]);
    if positions.len() == 4 {
        data.indices.extend([base + 1, base + 2, base + 3]);
    }
}

pub struct AssetSceneBuilder<'a> {
    vram: &'a Vram,
    meshes: &'a mut Assets<Mesh>,
    materials: &'a mut Assets<StandardMaterial>,
    images: &'a mut Assets<Image>,
    textures: BTreeMap<(u16, u16), Handle<Image>>,
}

impl<'a> AssetSceneBuilder<'a> {
    /// Retain bindings so a palette update can refresh image data while keeping
    /// resident meshes and materials. Each builder owns its texture cache.
    pub fn take_texture_bindings(&mut self) -> Vec<(u16, u16, Handle<Image>)> {
        std::mem::take(&mut self.textures)
            .into_iter()
            .map(|((page, clut), image)| (page, clut, image))
            .collect()
    }
    pub fn new(
        vram: &'a Vram,
        meshes: &'a mut Assets<Mesh>,
        materials: &'a mut Assets<StandardMaterial>,
        images: &'a mut Assets<Image>,
    ) -> Self {
        Self {
            vram,
            meshes,
            materials,
            images,
            textures: BTreeMap::new(),
        }
    }
    pub fn build_object(&mut self, object: &TmdObject, quarter: u8) -> Vec<MeshBinding> {
        let mut groups: BTreeMap<MaterialKey, MeshData> = BTreeMap::new();
        for polygon in &object.polygons {
            let key = (
                polygon.texture_page,
                polygon.clut,
                polygon.mode,
                polygon.flag,
            );
            append_polygon(groups.entry(key).or_default(), object, polygon, quarter);
        }
        let mut parts = Vec::new();
        for (key, data) in groups {
            let texture = match (key.0, key.1) {
                (Some(page), Some(clut)) => Some(
                    self.textures
                        .entry((page, clut))
                        .or_insert_with(|| {
                            let rgba = self
                                .vram
                                .texture_rgba(page, clut)
                                .expect("reviewed page must fit VRAM");
                            let mut image = Image::new(
                                Extent3d {
                                    width: 256,
                                    height: 256,
                                    depth_or_array_layers: 1,
                                },
                                TextureDimension::D2,
                                rgba,
                                TextureFormat::Rgba8UnormSrgb,
                                RenderAssetUsages::default(),
                            );
                            image.sampler = ImageSampler::nearest();
                            self.images.add(image)
                        })
                        .clone(),
                ),
                _ => None,
            };
            let mut mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, data.positions);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, data.normals);
            mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, data.colors);
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, data.uv);
            mesh.insert_indices(Indices::U32(data.indices));
            let material = StandardMaterial {
                base_color: Color::WHITE,
                base_color_texture: texture,
                unlit: key.3 & 1 != 0,
                cull_mode: if key.3 & 2 != 0 {
                    None
                } else {
                    Some(Face::Back)
                },
                alpha_mode: if key.2 & 2 != 0 {
                    AlphaMode::Blend
                } else {
                    AlphaMode::Mask(0.5)
                },
                ..default()
            };
            parts.push((self.meshes.add(mesh), self.materials.add(material)));
        }
        parts
    }
}
