#import bevy_pbr::{
    mesh_view_bindings::globals,
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    forward_io::{VertexOutput, FragmentOutput},
}

@group(2) @binding(100)
var<uniform> single_arrow_base_color: vec4<f32>;
@group(2) @binding(101)
var<uniform> double_arrow_base_color: vec4<f32>;
@group(2) @binding(102)
var<uniform> background_base_color: vec4<f32>;
@group(2) @binding(103)
var<uniform> number_of_tiles: f32;
@group(2) @binding(104)
var<uniform> forward_speed: f32;
@group(2) @binding(105)
var<uniform> backward_speed: f32;
@group(2) @binding(106)
var<uniform> bidirectional: u32;

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput{
    let is_bidirectional = (bidirectional == 1);

    let single_arrow_color = single_arrow_base_color.rgb;
    let double_arrow_color = double_arrow_base_color.rgb;
    let background_color = background_base_color.rgb;
    
    var is_forward = false;
    var is_backward = false;

    let side_margin = 0.1;
    let tile_length = 1.0 / number_of_tiles;
    let thickness = 0.2 / number_of_tiles;

    if in.uv.y > side_margin
        && in.uv.y < (1.0 - side_margin)
        && in.uv.x >= tile_length * 0.5
        && in.uv.x <= (1.0 - tile_length * 0.5)
    {
        let forward_progress = fract(globals.time * forward_speed * 0.5);
        is_forward = check_forward_pixel(
            in.uv.x,
            in.uv.y,
            forward_progress,
            thickness,
            number_of_tiles,
            tile_length,
        );

        if is_bidirectional & !is_forward {
            let backward_progress = fract(globals.time * backward_speed * 0.5);
            is_backward = check_backward_pixel(
                in.uv.x,
                in.uv.y,
                backward_progress,
                thickness,
                number_of_tiles,
                tile_length,
            );
        }
    }

    let is_arrow_pixel = is_forward || is_backward;
    let final_arrow_color = select(single_arrow_color, double_arrow_color, is_backward);
    let final_pixel_color = select(background_color, final_arrow_color, is_arrow_pixel);

    var pbr_input = pbr_input_from_standard_material(in, true);
    pbr_input.material.base_color = vec4<f32>(final_pixel_color, 1.0);

    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    return out;
}

fn check_forward_pixel (
    x: f32,
    y: f32,
    progress: f32,
    thickness: f32,
    number_of_tiles: f32,
    tile_length: f32,
) -> bool {
    var x_top = 0.5 - abs(y - 0.5) + progress;
    var overlap_x_top = 0.0;
    var overlap = false;

    if x_top > 1.0 {
        overlap = true;
        overlap_x_top = x_top;
        x_top -= 1.0;
    }

    x_top /= number_of_tiles;
    overlap_x_top /= number_of_tiles;

    for (var i = 0.0; i < number_of_tiles; i += 1.0) {
        let x_top_tile = (tile_length * i) + x_top;
        let overlap_x_top_tile = (tile_length * i) + overlap_x_top;


        if (x <= x_top_tile && x >= (x_top_tile - thickness))
            || (overlap && x <= overlap_x_top_tile && x >= (overlap_x_top_tile - thickness))
        {
            return true;
        }
    }
    return false;
}

fn check_backward_pixel (
    x: f32,
    y: f32,
    progress: f32,
    thickness: f32,
    number_of_tiles: f32,
    tile_length: f32,
) -> bool {
    var x_top = 0.5 + abs(y - 0.5) - progress;
    var overlap_x_top = 1.0;
    var overlap = false;

    if x_top < 0.0 {
        overlap = true;
        overlap_x_top = x_top;
        x_top += 1.0;
    }
    
    x_top /= number_of_tiles;
    overlap_x_top /= number_of_tiles;

    for (var i = 0.0; i < number_of_tiles; i += 1.0) {
        let x_top_tile = (tile_length * i) + x_top;
        let overlap_x_top_tile = (tile_length * i) + overlap_x_top;

        if (x >= x_top_tile && x <= (x_top_tile + thickness))
            || (overlap && x >= overlap_x_top_tile && x <= (overlap_x_top_tile + thickness))
        {
            return true;
        }
    }
    return false;
}