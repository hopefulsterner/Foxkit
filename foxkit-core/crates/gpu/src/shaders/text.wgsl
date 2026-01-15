// Foxkit text rendering shader
// Renders glyphs from atlas texture with per-vertex colors

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

struct Uniforms {
    viewport: vec2<f32>,
    _padding: vec2<f32>,
}

@group(0) @binding(0)
var glyph_atlas: texture_2d<f32>;
@group(0) @binding(1)
var glyph_sampler: sampler;
@group(0) @binding(2)
var<uniform> uniforms: Uniforms;

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    
    // Convert pixel coordinates to clip space (-1 to 1)
    let x = (input.position.x / uniforms.viewport.x) * 2.0 - 1.0;
    let y = 1.0 - (input.position.y / uniforms.viewport.y) * 2.0;
    
    output.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    output.uv = input.uv;
    output.color = input.color;
    
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let alpha = textureSample(glyph_atlas, glyph_sampler, input.uv).r;
    return vec4<f32>(input.color.rgb, input.color.a * alpha);
}
