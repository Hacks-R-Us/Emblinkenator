struct FrameData {
    frame: f32;
    frame_numerator: f32;
    frame_denominator: f32;
    seconds_elapsed: f32;
    whole_seconds_elapsed: f32;
};

struct LED {
    r: f32;
    g: f32;
    b: f32;
};

struct Result {
    leds: [[stride(12)]] array<LED>;
};

struct Coord {
    x: f32;
    y: f32;
    z: f32;
};

struct Positions {
    data: [[stride(12)]] array<Coord>;
};

struct NoiseData {
    size_x: u32;
    size_y: u32;
    size_z: u32;
    noise: array<f32>;
};

struct RGBValue {
    val: f32;
};

[[group(0), binding(0)]]
var<storage, read> params: FrameData;

[[group(0), binding(1)]]
var<storage, read> positions: Positions;

[[group(1), binding(0)]]
var<storage, read_write> result: Result;


[[group(2), binding(0)]]
var<storage, read> r_max: RGBValue;
[[group(2), binding(1)]]
var<storage, read> g_max: RGBValue;
[[group(2), binding(2)]]
var<storage, read> b_max: RGBValue;
[[group(2), binding(3)]]
var<storage, read> noise_data_r: NoiseData;
[[group(2), binding(4)]]
var<storage, read> noise_data_g: NoiseData;
[[group(2), binding(5)]]
var<storage, read> noise_data_b: NoiseData;

[[stage(compute), workgroup_size(64)]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {
    var index: u32 = global_id.x;
    var end: u32 = min(index + 64u, arrayLength(&result.leds));

    loop {
        var position: Coord = positions.data[index];
        // TODO: This assumes the vector has at least as many values as there are LEDs.
        let noise_r: f32 = noise_data_r.noise[u32((position.x * f32(noise_data_r.size_x)) + (position.y * f32(noise_data_r.size_y)) + position.z)];
        let noise_g: f32 = noise_data_g.noise[u32((position.x * f32(noise_data_g.size_x)) + (position.y * f32(noise_data_g.size_y)) + position.z)];
        let noise_b: f32 = noise_data_b.noise[u32((position.x * f32(noise_data_b.size_x)) + (position.y * f32(noise_data_b.size_y)) + position.z)];

        let value_r: f32 = noise_r * f32(r_max.val);
        let value_g: f32 = noise_g * f32(g_max.val);
        let value_b: f32 = noise_b * f32(b_max.val);

        result.leds[index].r = value_r;
        result.leds[index].g = value_g;
        result.leds[index].b = value_b;

        index = index + 1u;

        if (index >= end) {
            break;
        }
    }
}
