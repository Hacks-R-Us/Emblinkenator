[[block]]
struct FrameData {
  frame: f32;
  frameRate: f32;
};

struct LED {
    r: u32;
    g: u32;
    b: u32;
};

[[block]]
struct Result {
    leds: [[stride(12)]] array<LED>;
};

struct Coord {
    x: f32;
    y: f32;
    z: f32;
};

[[block]]
struct Positions {
    data: [[stride(12)]] array<Coord>;
};

[[block]]
struct NoiseData {
    noise: [[stride(4)]] array<f32>;
    size_x: u32;
    size_y: u32;
}

[[group(0), binding(0)]]
var<storage, read> params: FrameData;

[[group(0), binding(1)]]
var<storage, read> positions: Positions;

[[group(1), binding(0)]]
var<storage, read_write> result: Result;

[[group(2), binding(0)]]
var<storage, read> noise_data NoiseData;

fn get_pos (duration: f32) -> f32 {
    var pos: f32 = ((f32(1.0 / params.frameRate) * params.frame) % duration) / duration;

    pos = pos * 2.0;

    if (pos > 1.0) {
        pos = 1.0 - (pos - 1.0);
    }

    return pos;
}

[[stage(compute), workgroup_size(100)]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {
    var index: u32 = global_id.x;
    var end: u32 = index + 10u;

    var r_fade_duration: f32 = 10.0;
    var g_fade_duration: f32 = 3.0;
    var b_fade_duration: f32 = 2.0;

    var r_pos: f32 = get_pos(r_fade_duration);
    var g_pos: f32 = get_pos(g_fade_duration);
    var b_pos: f32 = get_pos(b_fade_duration);
    var r_max: f32 = 100.0;
    var g_max: f32 = 150.0;
    var b_max: f32 = 100.0;

    loop {
        result.leds[index].r = u32(floor(r_pos * r_max));
        result.leds[index].g = u32(floor(g_pos * g_max));
        result.leds[index].b = u32(floor(b_pos * b_max));

        index = index + 1u;

        if (index >= end) {
            break;
        }
    }
}
