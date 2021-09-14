use log::{debug, error};
use parking_lot::RwLock;
use std::collections::HashMap;

use tokio::sync::broadcast::{error::TryRecvError, Receiver};
use serde::Deserialize;

use crate::{frame_resolver::FrameResolverDataEvent, id::{DeviceId, FixtureId}, led::LED, state::ThreadedObject};

use super::{mqtt::MQTTSenderConfig, udp::UDPSenderConfig};

pub struct DeviceManager {
    devices: RwLock<HashMap<DeviceId, DeviceType>>,
    subscribed_events: SubscribedEvents,
    fixture_to_device: HashMap<FixtureId, DeviceId>,
}

pub enum DeviceType {
    LEDDataOutput(Box<dyn LEDDataOutput>),
}

#[derive(Deserialize)]
pub enum DeviceConfigType {
    LEDDataOutput(LEDOutputConfigType),
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Deserialize)]
pub enum LEDOutputConfigType {
    MQTT(MQTTSenderConfig),
    UDP(UDPSenderConfig)
}

pub trait LEDDataOutput: Send + Sync {
    fn on_frame(&self, frame: Vec<LED>);
}

pub enum DeviceManagerErrorAddDevice {
    DeviceAlreadyExists(DeviceId),
}

pub enum DeviceManagerErrorGetDevice {
    DeviceDoesNotExist(DeviceId),
}

struct SubscribedEvents {
    frame_resolver: Vec<Receiver<FrameResolverDataEvent>>,
}

enum DeviceManagerOpqueue {
    AddDevice(DeviceType),
    RemoveDevice(DeviceId),
}

impl DeviceManager {
    pub fn new() -> DeviceManager {
        DeviceManager {
            devices: RwLock::new(HashMap::new()),
            subscribed_events: SubscribedEvents {
                frame_resolver: vec![],
            },
            fixture_to_device: HashMap::new(),
        }
    }

    pub fn add_device(&self, id: DeviceId, device: DeviceType) {
        self.devices.write().insert(id, device);
    }

    pub fn remove_device(&self, _id: DeviceId) {}

    pub fn set_fixture_to_device(&mut self, fixture_id: FixtureId, device_id: DeviceId) {
        self.fixture_to_device.insert(fixture_id, device_id);
    }

    pub fn listen_to_resolved_frames(&mut self, recv: Receiver<FrameResolverDataEvent>) {
        self.subscribed_events.frame_resolver.push(recv);
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
            if let Some(device) = self.devices.read().get(device_id) {
                match device {
                    DeviceType::LEDDataOutput(device) => device.on_frame(event.data)
                }
            }
        }
    }
}
