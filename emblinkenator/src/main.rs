#![feature(drain_filter)]
#![feature(map_try_insert)]
#![deny(clippy::all)]
#![warn(clippy::perf)]
#![warn(clippy::complexity)]
#![deny(clippy::style)]
#![deny(clippy::print_stdout)]
#![deny(clippy::cast_lossless)]

use std::{
    fs,
    path::Path,
    sync::Arc,
    thread::{self, yield_now, JoinHandle},
    time::Duration,
};

use animation::AnimationTargetType;
use color_eyre::Report;

use config::{StartupAnimationTargetType, StartupConfig};
use devices::manager::DeviceManager;
use event_loop::GPUEventLoop;
use frame_resolver::FrameResolver;
use futures::executor::block_on;
use id::{AnimationId, AuxiliaryId, DeviceId, FixtureId, GroupId, InstallationId};
use log::debug;
use parking_lot::RwLock;
use pipeline::build_pipeline;
use state::ThreadedObject;
use world::{
    fixture::{Fixture, FixtureProps},
    Coord,
};

use crate::config::EmblinkenatorConfig;
use crate::{
    animation::manager::AnimationManager, auxiliary_data::manager::AuxiliaryDataManager,
    frame::FrameTimeKeeper,
};

use crate::state::EmblinkenatorState;
use crate::world::context::{WorldContext, WorldContextCollection};

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
        fs::write(
            "config.json",
            serde_json::to_string(&default_config).unwrap(),
        )
        .expect("Unable to create config file");
    }

    let config_json = fs::read_to_string("config.json").expect("Unable to read config file");
    let emblinkenator_config: EmblinkenatorConfig = serde_json::from_str(&config_json).unwrap();

    // Setup config
    let frame_numerator = emblinkenator_config.frame_numerator();
    let frame_denominator = emblinkenator_config.frame_denominator();
    let leds_per_compute_group = emblinkenator_config.leds_per_compute_group();

    // Create buffers for state transfer
    let (frame_resolver_buffer_sender, frame_resolver_buffer_receiver) =
        tokio::sync::broadcast::channel(emblinkenator_config.frame_buffer_size() as _);
    let (pipeline_context_buffer_sender, pipeline_context_buffer_receiver) =
        crossbeam::channel::bounded(emblinkenator_config.frame_buffer_size() as _);
    let (event_loop_frame_data_buffer_sender, event_loop_frame_data_buffer_receiver) =
        crossbeam::channel::bounded(1);

    // Collections
    let world_context_collection = WorldContextCollection::new();

    // Applies backpressure to move to next frame in time
    let frame_time_keeper = FrameTimeKeeper::new(
        frame_numerator,
        frame_denominator,
        u128::from(emblinkenator_config.frame_buffer_size),
    );
    frame_time_keeper.send_frame_data_to_blocking(
        "event_loop".to_string(),
        event_loop_frame_data_buffer_sender,
    );

    // Create state objects
    let world_context = Arc::new(RwLock::new(WorldContext::new(world_context_collection)));
    let animation_manager = Arc::new(RwLock::new(AnimationManager::new(
        &emblinkenator_config.shaders,
    )));
    let frame_resolver = FrameResolver::new(
        Arc::clone(&animation_manager),
        Arc::clone(&world_context),
        frame_resolver_buffer_receiver,
    );
    let device_manager = Arc::new(RwLock::new(DeviceManager::new()));
    let auxiliary_manager = AuxiliaryDataManager::new();
    let pipeline = build_pipeline(leds_per_compute_group).await;

    // Put objects behind RwLock if they're not already
    let frame_time_keeper = Arc::new(RwLock::new(frame_time_keeper));
    let frame_resolver = Arc::new(RwLock::new(frame_resolver));
    let auxiliary_manager = Arc::new(RwLock::new(auxiliary_manager));

    // State manager
    let mut state = EmblinkenatorState::new(
        Arc::clone(&animation_manager),
        Arc::clone(&auxiliary_manager),
        Arc::clone(&device_manager),
        Arc::clone(&frame_time_keeper),
        Arc::clone(&frame_resolver),
        Arc::clone(&world_context),
    );
    state.send_pipeline_context_to(pipeline_context_buffer_sender);

    let state = Arc::new(RwLock::new(state));

    let mut event_loop = GPUEventLoop::new(
        pipeline,
        event_loop_frame_data_buffer_receiver,
        pipeline_context_buffer_receiver,
        frame_resolver_buffer_sender,
    );

    let config_frame_resolver = Arc::clone(&frame_resolver);
    let config_device_manager = Arc::clone(&device_manager);
    let config_auxiliary_manager = Arc::clone(&auxiliary_manager);

    // Register objects with work loops
    let threaded_objects: Vec<Arc<RwLock<dyn ThreadedObject>>> = vec![
        frame_time_keeper,
        frame_resolver,
        state,
        device_manager,
        auxiliary_manager,
    ];
    let mut handles: Vec<JoinHandle<()>> = vec![];

    // Exists to make Rust compiler happy for now, should probably be linked to a stop button somewhere
    let running = true;

    for obj in threaded_objects {
        let handle = thread::spawn(move || 'work: loop {
            obj.write().tick();

            yield_now();

            if !running {
                break 'work;
            }
        });
        handles.push(handle);
    }

    // TODO: Remove this once event_loop is no longer async (when polling)
    handles.push(thread::spawn(move || 'work: loop {
        block_on(event_loop.tick());

        yield_now();

        if running {
            break 'work;
        }
    }));

    handles.push(thread::spawn(move || {
        let startup_config_json =
            fs::read_to_string("startup-config.json").expect("Unable to read startup config file");
        let startup_config: StartupConfig = serde_json::from_str(&startup_config_json).unwrap();
        for fixture in startup_config.fixtures {
            let mut positions = match fixture.led_positions {
                Some(positions) => positions,
                None => vec![Coord::origin(); fixture.num_leds as usize],
            };
            if positions.len() != fixture.num_leds as usize {
                positions = vec![Coord::origin(); fixture.num_leds as usize]
            }
            world_context
                .write()
                .add_fixture(Fixture::new(
                    FixtureId::new_from(fixture.id),
                    FixtureProps {
                        num_leds: fixture.num_leds,
                        led_positions: positions,
                    },
                ))
                .unwrap();
        }

        for animation in startup_config.animations {
            let target = match animation.target_id {
                StartupAnimationTargetType::Fixture(id) => {
                    AnimationTargetType::Fixture(FixtureId::new_from(id))
                }
                StartupAnimationTargetType::Installation(id) => {
                    AnimationTargetType::Installation(InstallationId::new_from(id))
                }
                StartupAnimationTargetType::Group(id) => {
                    AnimationTargetType::Group(GroupId::new_from(id))
                }
            };
            animation_manager
                .write()
                .create_animation(
                    AnimationId::new_from(animation.id),
                    animation.shader_id,
                    target,
                )
                .unwrap();
        }

        for startup_device in startup_config.devices {
            config_device_manager.write().add_device_from_config(
                DeviceId::new_from(startup_device.id.clone()),
                startup_device.config,
            );
        }

        for mapping in startup_config.fixtures_to_device {
            config_frame_resolver.write().set_fixture_to_device(
                FixtureId::new_from(mapping.0),
                DeviceId::new_from(mapping.1),
            );
        }

        // This horrible magic number brought to you by the lack of an API / database / migrations system.
        // We need to wait for the `on_device_added` events to be handled.
        thread::sleep(Duration::from_millis(1000));

        debug!("Adding auxiliaries");

        for auxiliary in startup_config.auxiliaries {
            config_auxiliary_manager
                .write()
                .add_auxiliary(
                    auxiliary.clone().into(),
                    auxiliary.clone().get_name(),
                    auxiliary.clone().into(),
                    auxiliary.clone().into(),
                )
                .unwrap_or_else(|_| {
                    panic!(
                        "Cannot add auxiliary {}",
                        Into::<AuxiliaryId>::into(auxiliary)
                    )
                });
        }

        for (animation_id, aux_ids) in startup_config.animation_auxiliary_sources {
            // Need the resulting Aux Ids
            let animation_id = AnimationId::new_from(animation_id);
            let aux_ids: Vec<AuxiliaryId> = aux_ids
                .iter()
                .map(|id| AuxiliaryId::new_from(id.to_string()))
                .collect();
            config_auxiliary_manager
                .write()
                .set_animation_auxiliary_sources_to(animation_id, aux_ids);
        }

        debug!("Setup complete");
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
