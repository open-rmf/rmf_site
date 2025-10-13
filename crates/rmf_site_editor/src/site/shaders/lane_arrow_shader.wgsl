#import bevy_pbr::{
    mesh_view_bindings::globals,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    forward_io::{VertexOutput, FragmentOutput},
}

struct BigF32 {
    @size(16)
    @align(16)
    value: f32,
}

struct BigU32 {
    @size(16)
    @align(16)
    value: u32,
}

struct LaneShaderSpeeds {
    forward: f32,
    backward: f32,
    _pad1: f32,
    _pad2: f32,
}

@group(2) @binding(100)
var<uniform> single_arrow_base_color: vec4<f32>;
@group(2) @binding(101)
var<uniform> double_arrow_base_color: vec4<f32>;
@group(2) @binding(102)
var<uniform> background_base_color: vec4<f32>;
@group(2) @binding(103)
var<uniform> number_of_arrows: BigF32;
@group(2) @binding(104)
var<uniform> speeds: LaneShaderSpeeds;
@group(2) @binding(105)
var<uniform> bidirectional: BigU32;
@group(2) @binding(106)
var<uniform> interacting: BigU32;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput{

    let is_bidirectional = (bidirectional.value == 1);
    let is_active = (interacting.value == 1);

    let single_arrow_color = single_arrow_base_color.rgb;
    let double_arrow_color = double_arrow_base_color.rgb;
    let background_color = background_base_color.rgb;

    var final_pixel_color = background_color;
    if !(is_bidirectional && !is_active) {
        var is_forward = false;
        var is_backward = false;

        let forward_arrow_width_ratio = 0.5;
        let backward_arrow_width_ratio = 0.5;

        let side_margin = 0.1;
        let end_margin = 1.0 / number_of_arrows.value;
        let thickness = 0.2 / number_of_arrows.value;

        var forward_progress = 0.0;
        var backward_progress = 0.0;

        if in.uv.y > side_margin
            && in.uv.y < (1.0 - side_margin)
            && in.uv.x > end_margin * 0.5
            && in.uv.x < (1.0 - end_margin * 0.5)
        {
            if is_active {
                forward_progress = fract(globals.time * speeds.forward);
            }

            if !is_bidirectional {
                let x_top = 0.5 - abs(in.uv.y - 0.5) + forward_progress;
                is_forward = check_forward_pixel(in.uv.x, x_top, thickness, number_of_arrows.value);

            } else {
                if in.uv.y < forward_arrow_width_ratio - (side_margin * 0.25) {
                    let y_val = in.uv.y - side_margin * 0.25;
                    let forward_width = forward_arrow_width_ratio * 0.5;
                    let x_top = forward_width - abs(y_val - forward_width) + forward_progress;

                    is_forward = check_forward_pixel(in.uv.x, x_top, thickness, number_of_arrows.value);
                }
                if in.uv.y > (1 - backward_arrow_width_ratio) + (side_margin * 0.25) && !is_forward {
                    if is_active {
                        backward_progress = fract(globals.time * speeds.backward);
                    }

                    let y_val = in.uv.y - (1 - backward_arrow_width_ratio) + side_margin * 0.25;
                    let backward_width = backward_arrow_width_ratio * 0.5;
                    let x_top = backward_width + abs(y_val - backward_width) - backward_progress;

                    is_backward = check_backward_pixel(in.uv.x, x_top, thickness, number_of_arrows.value);
                }
            }
        }

        let is_arrow_pixel = is_forward || is_backward;
        let final_arrow_color = select(single_arrow_color, double_arrow_color, is_backward);
        final_pixel_color = select(background_color, final_arrow_color, is_arrow_pixel);
    }

    var pbr_input = pbr_input_from_standard_material(in, true);
    pbr_input.material.base_color = vec4<f32>(final_pixel_color, 1.0);

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}

fn check_forward_pixel (
    x: f32,
    x_position: f32,
    thickness: f32,
    number_of_tiles: f32,
) -> bool {
    var x_top = x_position;

    var overlap_x_top = 0.0;
    var overlap = false;

    if x_top > 1.0 {
        overlap = true;
        overlap_x_top = x_top;
        x_top -= 1.0;
    }

    x_top /= number_of_tiles;
    overlap_x_top /= number_of_tiles;
    let tile_length = 1.0 / number_of_tiles;

    for (var i = 0.0; i < number_of_tiles; i = i + 1.0) {
        let x_top_tile = (tile_length * i) + x_top;
        let overlap_x_top_tile = (tile_length * i) + overlap_x_top;

        if ((x_top_tile >= x) && (x_top_tile <= x + thickness))
            || (overlap && (overlap_x_top_tile >= x) && (overlap_x_top_tile <= x + thickness))
        {
            return true;
        }
    }
    return false;
}

fn check_backward_pixel (
    x: f32,
    x_position: f32,
    thickness: f32,
    number_of_tiles: f32,
) -> bool {
    var x_top = x_position;

    var overlap_x_top = 1.0;
    var overlap = false;

    if x_top < 0.0 {
        overlap = true;
        overlap_x_top = x_top;
        x_top += 1.0;
    }

    x_top /= number_of_tiles;
    overlap_x_top /= number_of_tiles;
    let tile_length = 1.0 / number_of_tiles;

    for (var i = 0.0; i < number_of_tiles; i = i + 1.0) {
        let x_top_tile = (tile_length * i) + x_top;
        let overlap_x_top_tile = (tile_length * i) + overlap_x_top;

        if ((x >= x_top_tile) && (x <= x_top_tile + thickness))
            || (overlap && (x >= overlap_x_top_tile) && (x <= overlap_x_top_tile + thickness))
        {
            return true;
        }
    }
    return false;
}
