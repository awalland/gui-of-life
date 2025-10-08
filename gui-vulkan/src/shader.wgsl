struct GridVertexInput {
    @location(0) local_pos: vec2<f32>,
    @location(1) min: vec2<f32>,
    @location(2) max: vec2<f32>,
    @location(3) color: vec3<f32>,
};

struct UiVertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_grid(input: GridVertexInput) -> VertexOutput {
    let position = input.min + input.local_pos * (input.max - input.min);
    var out: VertexOutput;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.color = input.color;
    return out;
}

@vertex
fn vs_ui(input: UiVertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(input.position, 0.0, 1.0);
    out.color = input.color;
    return out;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(input.color, 1.0);
}
