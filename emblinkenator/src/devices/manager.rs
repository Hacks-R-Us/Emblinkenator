use log::{debug, error};
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};

use serde::Deserialize;
use tokio::sync::broadcast::{channel, error::TryRecvError, Receiver, Sender};

use crate::{
    auxiliary_data::{AuxiliaryDataType, AuxiliaryDataTypeConsumer},
    frame::FrameData,
    frame_resolver::FrameResolverDataEvent,
    id::{DeviceId, FixtureId},
    led::LED,
    state::{EmblinkenatorState, ThreadedObject},
};

use super::{
    auxiliary_data::{noise::NoiseAuxiliaryConfig, AuxiliaryDataDevice},
    led_output::{mqtt::MQTTSenderConfig, udp::UDPSenderConfig},
    threaded_device::{self, ThreadedDeviceWrapper},
};

pub struct DeviceManager {
    devices: RwLock<HashMap<DeviceId, Arc<ThreadedDeviceWrapper>>>,
    led_data_buffers: RwLock<HashMap<DeviceId, Sender<Vec<LED>>>>,
    subscribed_events: SubscribedEvents,
    event_emitters: RwLock<Vec<crossbeam::channel::Sender<DeviceManagerEvent>>>,
    fixture_to_device: HashMap<FixtureId, DeviceId>,
}

#[derive(Deserialize, Clone)]
pub enum DeviceConfigType {
    LEDDataOutput(LEDOutputConfigType),
    Auxiliary(AuxiliaryDataConfigType),
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Deserialize, Clone)]
pub enum LEDOutputConfigType {
    MQTT(MQTTSenderConfig),
    UDP(UDPSenderConfig),
}

#[derive(Deserialize, Clone)]
pub enum AuxiliaryDataConfigType {
    Noise(NoiseAuxiliaryConfig),
}

pub enum DeviceOutputType {
    Auxiliary(AuxiliaryDataTypeConsumer),
}

pub enum DeviceInputType {
    LEDData,
}

pub enum DeviceOutput {
    Auxiliary(AuxiliaryDataType),
}

pub enum DeviceInput {
    LEDData(Vec<LED>),
    FrameData(FrameData),
    NextFrameData(FrameData),
}

pub enum DeviceManagerErrorAddDevice {
    DeviceAlreadyExists(DeviceId),
}

pub enum DeviceManagerErrorGetDevice {
    DeviceDoesNotExist(DeviceId),
}

#[derive(Clone)]
pub enum DeviceManagerEvent {
    DeviceAdded(DeviceId),
    DeviceRemoved(DeviceId),
}

struct SubscribedEvents {
    frame_resolver: Vec<Receiver<FrameResolverDataEvent>>,
}

impl DeviceManager {
    pub fn new() -> DeviceManager {
        DeviceManager {
            devices: RwLock::new(HashMap::new()),
            led_data_buffers: RwLock::new(HashMap::new()),
            subscribed_events: SubscribedEvents {
                frame_resolver: vec![],
            },
            event_emitters: RwLock::new(vec![]),
            fixture_to_device: HashMap::new(),
        }
    }

    pub fn add_device_from_config(
        &mut self,
        config: DeviceConfigType,
        id: DeviceId,
        state: &EmblinkenatorState,
    ) {
        // Match type
        // Build devices
        // Link up buffers using state
    }

    pub fn get_device(&self, id: DeviceId) -> Option<Arc<ThreadedDeviceWrapper>> {
        self.devices.read().get(&id).map(|d| Arc::clone(d))
    }

    pub fn add_led_device(&self, id: DeviceId, mut device: Box<dyn LEDDataOutput>) {
        let (sender, receiver) = channel(10); // TODO: Get channel size from config
        device.set_data_buffer(receiver);

        let threaded_device = ThreadedDeviceWrapper::new(device);
        self.devices
            .write()
            .insert(id.clone(), Arc::new(threaded_device));
        self.led_data_buffers.write().insert(id.clone(), sender);

        self.emit_device_added(id);
    }

    pub fn add_auxiliary_device(&self, id: DeviceId, device: Box<dyn AuxiliaryDataDevice>) {
        let threaded_device = ThreadedDeviceWrapper::new(device);
        self.devices
            .write()
            .insert(id.clone(), Arc::new(threaded_device));

        self.emit_device_added(id);
    }

    pub fn remove_device(&self, _id: DeviceId) {}

    pub fn set_fixture_to_device(&mut self, fixture_id: FixtureId, device_id: DeviceId) {
        self.fixture_to_device.insert(fixture_id, device_id);
    }

    pub fn listen_to_resolved_frames(&mut self, recv: Receiver<FrameResolverDataEvent>) {
        self.subscribed_events.frame_resolver.push(recv);
    }

    pub fn subscribe_to_events(&mut self) -> crossbeam::channel::Receiver<DeviceManagerEvent> {
        let (sender, receiver) = crossbeam::channel::unbounded();
        self.event_emitters.write().push(sender);

        receiver
    }

    fn emit_device_added(&self, id: DeviceId) {
        self.emit_event(DeviceManagerEvent::DeviceAdded(id));
    }

    fn emit_event(&self, event: DeviceManagerEvent) {
        for sender in self.event_emitters.write().iter() {
            sender.send(event.clone()).ok();
        }
    }
}

impl ThreadedObject for DeviceManager {
    fn run(&mut self) {
        for event_subscriber in self.subscribed_events.frame_resolver.iter_mut() {
            let event = event_subscriber.try_recv();

            if event.is_err() {
                if let Err(err) = event {
                    match err {
                        TryRecvError::Lagged(messages) => {
                            error!(
                                "Device manager lagged behind pipeline by {} messages",
                                messages
                            );
                        }
                        TryRecvError::Closed => {
                            panic!("Frame resolver message channel closed");
                        }
                        TryRecvError::Empty => {}
                    }
                }
                continue;
            }

            let event = event.unwrap();

            debug!("{:?}", event);

            let device_id = self.fixture_to_device.get(&event.target);
            if device_id.is_none() {
                continue;
            }
            let device_id = device_id.unwrap();
            if let Some(sender) = self.led_data_buffers.read().get(device_id) {
                sender.send(event.data).ok();
            }
        }
    }
}
