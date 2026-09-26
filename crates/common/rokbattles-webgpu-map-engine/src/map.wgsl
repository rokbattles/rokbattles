// Vertex locations must match Vertex and the layout in gpu.rs. The CPU converts
// world coordinates to clip space so tiles and CSS-sized annotations share a pass.
struct Output {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) stroke: vec4<f32>,
}

@group(0) @binding(0) var image: texture_2d<f32>;
@group(0) @binding(1) var image_sampler: sampler;

@vertex
fn vs(
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) stroke: vec4<f32>,
) -> Output {
    var out: Output;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.uv = uv;
    out.color = color;
    out.stroke = stroke;
    return out;
}

@fragment
fn fs(in: Output) -> @location(0) vec4<f32> {
    // Images use a tint, solid geometry a white texel, and ASCII text an atlas.
    // Keep RGB unpremultiplied: ALPHA_BLENDING applies source alpha in the pipeline.
    let sampled = textureSample(image, image_sampler, in.uv) * in.color;
    let diagonal = (in.position.x + in.position.y) / 16.0;
    // Evaluate derivatives before the geometry branches so both backends see
    // uniform control flow and can antialias stripe edges consistently.
    let edge = fwidth(diagonal);
    let stripe = 1.0 - smoothstep(0.35 - edge, 0.35 + edge, fract(diagonal));
    // Stroke distances are in CSS pixels, preserving widths and 6-on/4-off
    // dashes across zoom and device pixel ratios. Derivatives give the actual
    // framebuffer pixel footprint, including fractional positions on either axis.
    let stroke_pixel = max(fwidth(in.stroke.xy), vec2<f32>(0.0001));
    let coverage = clamp((in.stroke.z - abs(in.stroke.y)) / stroke_pixel.y + 0.5, 0.0, 1.0);
    let dash_distance = abs(fract((in.stroke.x + 2.0) / 10.0) * 10.0 - 5.0);
    let dash = clamp((3.0 - dash_distance) / stroke_pixel.x + 0.5, 0.0, 1.0);
    if in.stroke.z > 0.0 {
        return vec4<f32>(in.color.rgb, in.color.a * coverage * select(1.0, dash, in.stroke.w > 0.5));
    }
    // Negative UV selects forbidden hatching.
    if in.uv.x < 0.0 {
        return vec4<f32>(in.color.rgb, in.color.a * (0.2 + 0.8 * stripe));
    }
    return sampled;
}

struct PresentationOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_present(@builtin(vertex_index) index: u32) -> PresentationOutput {
    // A single oversized triangle covers the canvas without a diagonal seam.
    let positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let position = positions[index];
    var out: PresentationOutput;
    out.position = vec4<f32>(position, 0.0, 1.0);
    out.uv = position * vec2<f32>(0.5, -0.5) + vec2<f32>(0.5);
    return out;
}

@fragment
fn fs_present(in: PresentationOutput) -> @location(0) vec4<f32> {
    // The map has already blended in display space. Undo sRGB encoding here;
    // the sRGB surface restores it on write, preserving the composited colors.
    let color = textureSample(image, image_sampler, in.uv);
    let linear = select(
        pow((color.rgb + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4)),
        color.rgb / 12.92,
        color.rgb <= vec3<f32>(0.04045),
    );
    return vec4<f32>(linear, color.a);
}
