// Foxkit quad/primitive shader

struct Uniforms {
    viewport: vec2<f32>,
    _padding: vec2<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) uv: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    
    // Convert from pixel coordinates to clip space (-1 to 1)
    let x = (input.position.x / uniforms.viewport.x) * 2.0 - 1.0;
    let y = 1.0 - (input.position.y / uniforms.viewport.y) * 2.0;
    
    output.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    output.color = input.color;
    output.uv = input.uv;
    
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}
