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
    size_x: u32;
    size_y: u32;
    size_z: u32;
    noise: array<f32>;
};

[[group(0), binding(0)]]
var<storage, read> params: FrameData;

[[group(0), binding(1)]]
var<storage, read> positions: Positions;

[[group(1), binding(0)]]
var<storage, read_write> result: Result;

[[group(2), binding(0)]]
var<storage, read> noise_data_r: NoiseData;

[[group(2), binding(1)]]
var<storage, read> noise_data_g: NoiseData;

[[group(2), binding(2)]]
var<storage, read> noise_data_b: NoiseData;

[[stage(compute), workgroup_size(64)]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {
    var index: u32 = global_id.x;
    var end: u32 = min(index + 64u, arrayLength(&result.leds));

    loop {
        var position: Coord = positions.data[index];
        result.leds[index].r = noise_data_r.noise;
        result.leds[index].g = noise_data_g.noise;
        result.leds[index].b = noise_data_b.noise;

        index = index + 1u;

        if (index >= end) {
            break;
        }
    }
}
