// AuraLite Powder WGPU shader - nearest-neighbour sample of the particle texture
// with the same camera transform used by the CPU composer.

struct Uniforms {
    grid_width: u32,
    grid_height: u32,
    surface_width: u32,
    surface_height: u32,
    scale: f32,
    offset_x: f32,
    offset_y: f32,
    _pad: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var particle_texture: texture_2d<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Fullscreen triangle
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -3.0),
        vec2<f32>(3.0, 1.0),
        vec2<f32>(-1.0, 1.0)
    );
    var uv = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 2.0),
        vec2<f32>(2.0, 0.0),
        vec2<f32>(0.0, 0.0)
    );
    var out: VertexOutput;
    out.position = vec4<f32>(pos[vertex_index], 0.0, 1.0);
    out.uv = uv[vertex_index];
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let screen = in.uv * vec2<f32>(f32(uniforms.surface_width), f32(uniforms.surface_height));
    let world = screen / max(uniforms.scale, 0.0001)
        + vec2<f32>(uniforms.offset_x, uniforms.offset_y);
    let gx = i32(floor(world.x));
    let gy = i32(floor(world.y));
    if (gx < 0 || gy < 0 || gx >= i32(uniforms.grid_width) || gy >= i32(uniforms.grid_height)) {
        return vec4<f32>(0.02, 0.02, 0.04, 1.0);
    }
    return textureLoad(particle_texture, vec2<i32>(gx, gy), 0);
}

@fragment
fn fs_heatmap(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = fs_main(in);
    let glow = color.r * 0.5;
    return vec4<f32>(color.rgb + vec3<f32>(glow, glow * 0.5, 0.0), 1.0);
}
