#![feature(drain_filter)]
#![feature(map_try_insert)]

#![deny(clippy::all)]
#![warn(clippy::perf)]
#![warn(clippy::complexity)]
#![deny(clippy::style)]
#![deny(clippy::print_stdout)]
#![deny(clippy::cast_lossless)]

use std::{fs, path::Path, sync::Arc, thread::{self, JoinHandle}};

use color_eyre::Report;

use devices::manager::DeviceManager;
use event_loop::GPUEventLoop;
use frame_resolver::FrameResolver;
use futures::executor::block_on;
use parking_lot::RwLock;
use pipeline::build_pipeline;
use state::ThreadedObject;

use crate::{animation::{manager::AnimationManager, AnimationTargetType}, config::{EmblinkenatorConfig, StartupAnimationTargetType, StartupConfig}, devices::{manager::{DeviceConfigType, DeviceType, LEDOutputConfigType}, mqtt::MQTTSender, udp::UDPSender}, id::{DeviceId, FixtureId, InstallationId, GroupId}, state::EmblinkenatorState, world::{Coord, context::{WorldContext, WorldContextCollection}, fixture::{Fixture, FixtureProps}}};

#[macro_use]
extern crate protected_id;

mod animation;
mod config;
mod devices;
mod event_loop;
mod events;
mod frame;
mod frame_resolver;
mod id;
mod led;
mod opqueue;
mod pipeline;
mod state;
mod world;

// TODO: Set workgroup_size as override constant (blocked, track https://github.com/gfx-rs/wgpu/issues/1762)
// TODO: Open-source!

#[tokio::main]
async fn main() {
    setup_logging().unwrap();

    if !Path::exists(Path::new("config.json")) {
        let default_config = EmblinkenatorConfig::new(25, 10, 100);
        fs::write("config.json", serde_json::to_string(&default_config).unwrap()).expect("Unable to create config file");
    }

    let config_json = fs::read_to_string("config.json").expect("Unable to read config file");
    let emblinkenator_config: EmblinkenatorConfig = serde_json::from_str(&config_json).unwrap();

    // Setup config
    let frame_rate = emblinkenator_config.frame_rate();
    let leds_per_compute_group = emblinkenator_config.leds_per_compute_group();

    // Create buffers for state transfer
    let (frame_resolver_buffer_sender, frame_resolver_buffer_reciever) =
        crossbeam::channel::bounded(emblinkenator_config.frame_buffer_size() as _);
    let (pipeline_context_buffer_sender, pipeline_context_buffer_reciever) =
        crossbeam::channel::bounded(emblinkenator_config.frame_buffer_size() as _);

    // Collections
    let world_context_collection = WorldContextCollection::new();

    // Create state objects
    let world_context = Arc::new(RwLock::new(WorldContext::new(world_context_collection)));
    let animation_manager = Arc::new(RwLock::new(AnimationManager::new()));
    let frame_resolver = FrameResolver::new(
        Arc::clone(&animation_manager),
        Arc::clone(&world_context),
        frame_resolver_buffer_reciever,
    );
    let device_manager = DeviceManager::new(Arc::clone(&world_context));
    let pipeline = build_pipeline(leds_per_compute_group).await;

    // Put objects behind RwLock if they're not already
    let frame_resolver = Arc::new(RwLock::new(frame_resolver));
    let device_manager = Arc::new(RwLock::new(device_manager));

    // State manager
    let state = EmblinkenatorState::new(
        Arc::clone(&animation_manager),
        Arc::clone(&world_context),
        pipeline_context_buffer_sender,
    );

    let state = Arc::new(RwLock::new(state));

    // Subscribe to events
    {
        device_manager
            .write()
            .listen_to_resolved_frames(frame_resolver.read().subscribe_to_resolved_frames());
    }

    // Temp, setup stuff from config
    {
        let startup_config_json = fs::read_to_string("startup_config.json").expect("Unable to read startup config file");
        let startup_config: StartupConfig = serde_json::from_str(&startup_config_json).unwrap();
        for fixture in startup_config.fixtures {
            let positions = match fixture.led_positions {
                Some(positions) => positions,
                None => vec![Coord::origin(); fixture.num_leds as usize]
            };
            world_context.write().add_fixture(
                Fixture::new(
                    FixtureId::new_from(fixture.id),
                    FixtureProps {
                        num_leds: fixture.num_leds,
                        led_positions: positions
                    }
                )
            ).unwrap();
        }

        for animation in startup_config.animations {
            let target = match animation.target_id {
                StartupAnimationTargetType::Fixture(id) => AnimationTargetType::Fixture(FixtureId::new_from(id)),
                StartupAnimationTargetType::Installation(id) => AnimationTargetType::Installation(InstallationId::new_from(id)),
                StartupAnimationTargetType::Group(id) => AnimationTargetType::Group(GroupId::new_from(id)),
            };
            animation_manager.write().create_animation(animation.shader_id, target).unwrap();
        }

        for startup_device in startup_config.devices {
            match startup_device.config {
                DeviceConfigType::LEDDataOutput(output) => match output {
                        LEDOutputConfigType::MQTT(config) => {
                            let mqtt_device = MQTTSender::new(DeviceId::new_from(startup_device.id.clone()), config);
                            device_manager.write().add_device(
                                DeviceId::new_from(startup_device.id.clone()),
                                DeviceType::LEDDataOutput(Box::new(mqtt_device))
                            );
                        },
                        LEDOutputConfigType::UDP(config) => {
                            let udp_device = UDPSender::new(DeviceId::new_from(startup_device.id.clone()), config);
                            device_manager.write().add_device(
                                DeviceId::new_from(startup_device.id.clone()),
                                DeviceType::LEDDataOutput(Box::new(udp_device))
                            );
                        }
                }
            };
        }

        for mapping in startup_config.fixtures_to_device {
            device_manager.write().set_fixture_to_device(FixtureId::new_from(mapping.0), DeviceId::new_from(mapping.1));
        }
    }

    let mut event_loop = GPUEventLoop::new(
        pipeline,
        frame_rate,
        pipeline_context_buffer_reciever,
        frame_resolver_buffer_sender,
    );

    // Register objects with work loops
    let threaded_objects: Vec<Arc<RwLock<dyn ThreadedObject>>> =
        vec![frame_resolver, state, device_manager];
    let mut handles: Vec<JoinHandle<()>> = vec![];

    // Exists to make Rust compiler happy for now, should probably be linked to a stop button somewhere
    let running = true;

    for obj in threaded_objects {
        let handle = thread::spawn(move || 'work: loop {
            obj.write().run();

            if !running {
                break 'work;
            }
        });
        handles.push(handle);
    }

    // TODO: Remove this once event_loop is no longer async (when polling)
    handles.push(thread::spawn(move || 'work: loop {
        block_on(event_loop.run());

        if running {
            break 'work;
        }
    }));

    for handle in handles {
        handle.join().unwrap();
    }
}

fn setup_logging() -> Result<(), Report> {
    if std::env::var("RUST_LIB_BACKTRACE").is_err() {
        std::env::set_var("RUST_LIB_BACKTRACE", "1");
    }
    color_eyre::install()?;

    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "emblinkenator=info");
    }
    env_logger::init();

    Ok(())
}
