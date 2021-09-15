# Emblinkenator

It makes the LEDs go blink so you don't have to!

## The Emblinkenator Model of The World

Firstly, this is not a perfect model of every system and does not claim to be. With that out of the way, Emblinkenator divides the world into 4 categories:

- An `LED` - the single base unit. It has a position relative to the `Fixture` it's attached to.
- A `Fixture` - A hardware device with a string of `LED`s attached (at the moment limited to one string of `LED`s per `Fixture`). `LED`s are assumed to be connected in sequence such that `LED` 0 is closest to the data pin. A `Fixture` has a position relative to its parent, which may be the world or an `Installation`.
- An `Installation` - A collection of `Fixtures` that comprise a larger display (e.g. Hubs on a Dome). Each `Fixture` then has a position relative to the `Installation`. The `Installation` has a position relative to its parent, either the world or a `Group`.
- A `Group` - A collection of other `Group`s and `Installation`s. A group has a position relative to the world origin.

Some rules apply:
- A `Fixture` can only belong to one `Installation`.
- An `Installation` can only belong to one group.
- A `Group` can only belong to one group.

Emblinkenator then maps an `Animation` onto either a `Fixture`, an `Installation`, or a `Group`.

An `Animation` is an instance of a shader written in WGSL that produces RGB values for every `LED` in the target. The `Animation` has access to information about the positions of `LED`s within the target, the frame rate of the system, the current time, etc. Future work will allow auxiliary data to be provided to an `Animation`, e.g. data from a sensor.

A `Fixture` is then mapped to some hardware device such as an Arduino. Mappings to hardware devices are changeable on-the-fly, it is assumed that the target device has the right number of `LED`s corresponding to the `Fixture` representation, or at least can handle recieving the wrong number of RGB values.

## Requirements

- Rust
- Some hardware with some LEDs attached

## Getting Started

First, get set up with a Rust environment (Note: Right now WSL support is likely to be spotty due to limited access to GPU resources).

Then create a file called `startup_config.json` in the root directory of this project. Inside the `emblinkenator` directory there is a file called `startup_config.json.example`, copy the contents of this file to `startup_config.json` and change the settings to match your needs. This is a temporary measure until a UI is developed, it will create the initial configuration of the system. Note that the `led_positions` value is optional, if omitted all LED positions will be set to the world origin (0, 0, 0).

Next, or meanwhile, flash your hardware with some code that can recieve a buffer of (X) RGB values where (X) is the number of LEDs attached to the hardware. At the moment the supported transport streams are a direct UDP connection, or a MQTT connection via a third-party broker. Example implementations for both are available for the NodeMCU platform in the `examples/hardware` directory.

Finally, run `cargo run`, and you should get some blinking LEDs! Or, you may get a Segfault.

### Segfault

WebGPU needs a [very recent](https://github.com/gfx-rs/wgpu/issues/1906#issuecomment-913071836) version of Vulkan in order to use the Vulkan backend, if that's not available for your system (or you don't fancy installing it) you can force Emblinkenator to use OpenGL as the backend by setting the `WGPU_BACKEND` environment variable to `gl` e.g. `WGPU_BACKEND=gl cargo run`. This may have a performance impact.

## Other Notes

Logging is controlled by the `RUST_LOG` environment variable. If it is not set, the default is `emblinkenator=info`. Setting this to `emblinkenator=debug` will print debug messages for Emblinkenator only, setting it to `debug` will print debug statements from all linked Rust libraries (you have been warned!).

On first run, the `config.json` file will be generated. In there you can change the framerate (default is 25) and frame buffer size (default is 10).
