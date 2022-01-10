#![feature(drain_filter)]
#![feature(map_try_insert)]
#![feature(trait_upcasting)]

#![deny(clippy::all)]
#![warn(clippy::perf)]
#![warn(clippy::complexity)]
#![deny(clippy::style)]
#![deny(clippy::print_stdout)]
#![deny(clippy::cast_lossless)]

use std::{fs, path::Path, sync::Arc, thread::{self, JoinHandle, sleep}, time::Duration};

use color_eyre::Report;

use devices::manager::DeviceManager;
use event_loop::GPUEventLoop;
use frame_resolver::FrameResolver;
use futures::executor::block_on;
use parking_lot::RwLock;
use pipeline::build_pipeline;
use state::ThreadedObject;

use crate::{animation::{manager::AnimationManager}, auxiliary_data::AuxiliaryDataManager, frame::FrameTimeKeeper};
use crate::config::{EmblinkenatorConfig};


use crate::state::EmblinkenatorState;
use crate::world::{context::{WorldContext, WorldContextCollection}};

#[macro_use]
extern crate protected_id_derive;

mod animation;
mod auxiliary_data;
mod config;
mod devices;
mod event_loop;
mod events;
mod frame;
mod frame_resolver;
mod id;
mod led;
mod pipeline;
mod state;
mod world;

// TODO: Set workgroup_size as override constant (blocked, track https://github.com/gfx-rs/wgpu/issues/1762)

#[tokio::main]
async fn main() {
    setup_logging().unwrap();

    if !Path::exists(Path::new("config.json")) {
        let default_config = EmblinkenatorConfig::default();
        fs::write("config.json", serde_json::to_string(&default_config).unwrap()).expect("Unable to create config file");
    }

    let config_json = fs::read_to_string("config.json").expect("Unable to read config file");
    let emblinkenator_config: EmblinkenatorConfig = serde_json::from_str(&config_json).unwrap();

    // Setup config
    let frame_rate = emblinkenator_config.frame_rate();
    let leds_per_compute_group = emblinkenator_config.leds_per_compute_group();

    // Create buffers for state transfer
    let (frame_resolver_buffer_sender, frame_resolver_buffer_reciever) =
        tokio::sync::broadcast::channel(emblinkenator_config.frame_buffer_size() as _);
    let (pipeline_context_buffer_sender, pipeline_context_buffer_reciever) =
        crossbeam::channel::bounded(emblinkenator_config.frame_buffer_size() as _);
    let (event_loop_frame_data_buffer_sender, event_loop_frame_data_buffer_receiver) = crossbeam::channel::bounded(1);

    // Collections
    let world_context_collection = WorldContextCollection::new();

    // Applies backpressure to move to next frame in time
    let frame_time_keeper = FrameTimeKeeper::new(frame_rate, u128::from(emblinkenator_config.frame_buffer_size));
    frame_time_keeper.send_frame_data_to(event_loop_frame_data_buffer_sender);

    // Create state objects
    let world_context = Arc::new(RwLock::new(WorldContext::new(world_context_collection)));
    let animation_manager = Arc::new(RwLock::new(AnimationManager::new(&emblinkenator_config.shaders)));
    let frame_resolver = FrameResolver::new(
        Arc::clone(&animation_manager),
        Arc::clone(&world_context),
        frame_resolver_buffer_reciever,
    );
    let device_manager = Arc::new(RwLock::new(DeviceManager::new()));
    let auxiliary_manager = AuxiliaryDataManager::new(Arc::clone(&device_manager));
    let pipeline = build_pipeline(leds_per_compute_group).await;

    // Put objects behind RwLock if they're not already
    let frame_time_keeper = Arc::new(RwLock::new(frame_time_keeper));
    let frame_resolver = Arc::new(RwLock::new(frame_resolver));
    let auxiliary_manager = Arc::new(RwLock::new(auxiliary_manager));

    // State manager
    let mut state = EmblinkenatorState::new(
        Arc::clone(&animation_manager),
        Arc::clone(&auxiliary_manager),
        Arc::clone(&frame_time_keeper),
        Arc::clone(&world_context),
    );
    state.send_pipeline_context_to(pipeline_context_buffer_sender);

    let state = Arc::new(RwLock::new(state));

    // Subscribe to events
    {
        device_manager
            .write()
            .listen_to_resolved_frames(frame_resolver.read().subscribe_to_resolved_frames());
    }

    // Temp, setup stuff from config
    {
        /*let startup_config_json = fs::read_to_string("startup_config.json").expect("Unable to read startup config file");
        let startup_config: StartupConfig = serde_json::from_str(&startup_config_json).unwrap();
        for fixture in startup_config.fixtures {
            let mut positions = match fixture.led_positions {
                Some(positions) => positions,
                None => vec![Coord::origin(); fixture.num_leds as usize]
            };
            if positions.len() != fixture.num_leds as usize {
                positions = vec![Coord::origin(); fixture.num_leds as usize]
            }
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
                            device_manager.write().add_led_device(
                                DeviceId::new_from(startup_device.id.clone()),
                                Box::new(mqtt_device)
                            );
                        },
                        LEDOutputConfigType::UDP(config) => {
                            let udp_device = UDPSender::new(DeviceId::new_from(startup_device.id.clone()), config);
                            device_manager.write().add_led_device(
                                DeviceId::new_from(startup_device.id.clone()),
                                Box::new(udp_device)
                            );
                        }
                }
                DeviceConfigType::Auxiliary(aux) => match aux {
                    AuxiliaryDataConfigType::Noise(config) => {
                        let noise_aux = NoiseAuxiliaryDataDevice::new(DeviceId::new_from(startup_device.id.clone()), config);
                        device_manager.write().add_auxiliary_device(DeviceId::new_from(startup_device.id.clone()), Box::new(noise_aux));
                    },
                },
            };
        }

        for mapping in startup_config.fixtures_to_device {
            device_manager.write().set_fixture_to_device(FixtureId::new_from(mapping.0), DeviceId::new_from(mapping.1));
        }*/
    }

    let mut event_loop = GPUEventLoop::new(
        pipeline,
        event_loop_frame_data_buffer_receiver,
        pipeline_context_buffer_reciever,
        frame_resolver_buffer_sender,
    );

    // Register objects with work loops
    let threaded_objects: Vec<Arc<RwLock<dyn ThreadedObject>>> =
        vec![frame_time_keeper, frame_resolver, state, device_manager];
    let mut handles: Vec<JoinHandle<()>> = vec![];

    // Exists to make Rust compiler happy for now, should probably be linked to a stop button somewhere
    let running = true;

    for obj in threaded_objects {
        let handle = thread::spawn(move || 'work: loop {
            obj.write().run();

            sleep(Duration::from_millis(1));

            if !running {
                break 'work;
            }
        });
        handles.push(handle);
    }

    // TODO: Remove this once event_loop is no longer async (when polling)
    handles.push(thread::spawn(move || 'work: loop {
        block_on(event_loop.run());

        sleep(Duration::from_millis(1));

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
