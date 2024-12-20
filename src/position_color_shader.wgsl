// Vertex
struct CameraUniform {
    view_proj: mat4x4<f32>,
};

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) vert_pos: vec3<f32>,
}

@vertex
fn vs_main(
    @builtin(vertex_index) in_vertex_index: u32,
) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(1  - i32(in_vertex_index)) * 0.5;
    let y = f32(i32(in_vertex_index & 1u) * 2 - 1) * 0.5;
    out.clip_position = camera.view_proj * vec4<f32>(x, y, 0.0, 1.0);
    out.vert_pos = out.clip_position.xyz;
    return out;
}

// Fragment
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var pos = in.vert_pos / 2.0  + 1.0;
    pos = pos / 10.0; // TODO: improve scaling
    return vec4<f32>(pos.xyz, 1.0);
}

