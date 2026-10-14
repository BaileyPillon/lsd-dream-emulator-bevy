import bevy_pbr::render::forward_io::VertexOutput;
import bevy_render::color_operations::{linear_rgb_to_srgb,srgb_to_linear_rgb};

struct BoxUniform {
    counts:vec4<u32>,
    rectangles:array<vec4<i32>,128>,
    colors:array<vec4<u32>,128>,
}
@group(3) @binding(0) var<uniform> boxes:BoxUniform;
@group(3) @binding(1) var scene:texture_2d<f32>;
@group(3) @binding(2) var scene_sampler:sampler;

@fragment
fn fragment(input:VertexOutput)->@location(0) vec4<f32> {
    let uv=input.uv;
    let sampled=textureSample(scene,scene_sampler,uv);
    let display=clamp(linear_rgb_to_srgb(sampled.rgb),vec3<f32>(0.0),vec3<f32>(1.0));
    var back=vec3<u32>(round(display*255.0))>>vec3<u32>(3u);
    let pixel=vec2<i32>(floor(uv*vec2<f32>(boxes.counts.yz)));
    for (var index=0u;index<boxes.counts.x;index+=1u) {
        let rect=boxes.rectangles[index];
        if (all(pixel>=rect.xy) && all(pixel<rect.xy+rect.zw)) {
            let color=boxes.colors[index];
            let front=color.xyz;
            if ((color.w&4u)==0u) {back=front;}
            else {
                switch (color.w&3u) {
                    case 0u:{back=(back+front)>>vec3<u32>(1u);}
                    case 1u:{back=min(back+front,vec3<u32>(31u));}
                    case 2u:{back=back-min(back,front);}
                    default:{back=min(back+(front>>vec3<u32>(2u)),vec3<u32>(31u));}
                }
            }
        }
    }
    let expanded=(back<<vec3<u32>(3u))|(back>>vec3<u32>(2u));
    return vec4<f32>(srgb_to_linear_rgb(vec3<f32>(expanded)/255.0),1.0);
}
