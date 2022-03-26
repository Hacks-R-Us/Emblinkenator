struct FrameData {
  frame: f32;
  frame_rate: f32;
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

[[group(0), binding(0)]]
var<storage, read> params: FrameData;

[[group(0), binding(1)]]
var<storage, read> positions: Positions;

[[group(1), binding(0)]]
var<storage, read_write> result: Result;

[[stage(compute), workgroup_size(64)]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {
    var index: u32 = global_id.x;
    var end: u32 = min(index + 64u, arrayLength(&result.leds));

    loop {
        result.leds[index].r = positions.data[index].x;
        result.leds[index].g = positions.data[index].y;
        result.leds[index].b = positions.data[index].z;

        index = index + 1u;

        if (index >= end) {
            break;
        }
    }
}
