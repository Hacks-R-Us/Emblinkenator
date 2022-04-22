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

struct StepValue {
    val: f32;
};

struct RGBValue {
    val: f32;
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

[[group(2), binding(0)]]
var<storage, read> step_per_sec: StepValue;
[[group(2), binding(1)]]
var<storage, read> red: RGBValue;
[[group(2), binding(2)]]
var<storage, read> green: RGBValue;
[[group(2), binding(3)]]
var<storage, read> blue: RGBValue;

[[stage(compute), workgroup_size(64)]]
fn main([[builtin(global_invocation_id)]] global_id: vec3<u32>) {
    var num_leds = arrayLength(&result.leds);
    var index: u32 = global_id.x;
    var end: u32 = min(index + 64u, num_leds);

    var time_step = 1.0 / step_per_sec.val;
    var total_time = time_step * f32(num_leds);

    var target = u32((params.seconds_elapsed - (floor(params.seconds_elapsed / total_time) * total_time)) / time_step);

    if (target >= index && target < end) {
        result.leds[target].r = red.val;
        result.leds[target].g = green.val;
        result.leds[target].b = blue.val;
    }
}
