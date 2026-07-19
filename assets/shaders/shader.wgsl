// AuraLite Powder WGPU shader - simple fullscreen triangle rendering particle colors
// Storage texture containing particle colors

struct Uniforms {
    width: u32,
    height: u32,
    scale: f32,
    offset_x: f32,
    offset_y: f32,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var particle_texture: texture_storage_2d<rgba8unorm, read>;
@group(0) @binding(2) var tex_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Fullscreen triangle trick
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
    // Convert UV to world coords considering camera
    let screen_pos = in.uv; // 0..1
    let world_x = u32((screen_pos.x * f32(uniforms.width) + uniforms.offset_x));
    let world_y = u32((screen_pos.y * f32(uniforms.height) + uniforms.offset_y));

    if (world_x >= uniforms.width || world_y >= uniforms.height) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }

    let color = textureLoad(particle_texture, vec2<u32>(world_x, world_y));
    // Apply temperature glow post-process (optional)
    // For now just return color
    return color;
}

@fragment
fn fs_heatmap(in: VertexOutput) -> @location(0) vec4<f32> {
    let screen_pos = in.uv;
    let world_x = u32(screen_pos.x * f32(uniforms.width));
    let world_y = u32(screen_pos.y * f32(uniforms.height));
    let color = textureLoad(particle_texture, vec2<u32>(world_x, world_y));
    // Fake heat glow based on red channel intensity
    let glow = color.r * 0.5;
    return vec4<f32>(color.rgb + vec3<f32>(glow, glow*0.5, 0.0), 1.0);
}
