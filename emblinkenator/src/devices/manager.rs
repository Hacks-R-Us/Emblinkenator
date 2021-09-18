use log::{debug, error};
use parking_lot::RwLock;
use std::{collections::HashMap};

use tokio::sync::broadcast::{Receiver, Sender, channel, error::TryRecvError};
use serde::Deserialize;

use crate::{frame_resolver::FrameResolverDataEvent, id::{DeviceId, FixtureId}, led::LED, state::ThreadedObject};

use super::{mqtt::MQTTSenderConfig, threaded_device::{ThreadedDevice, ThreadedDeviceWrapper}, udp::UDPSenderConfig};

pub struct DeviceManager {
    devices: RwLock<HashMap<DeviceId, ThreadedDeviceWrapper>>,
    led_data_buffers: RwLock<HashMap<DeviceId, Sender<Vec<LED>>>>,
    subscribed_events: SubscribedEvents,
    fixture_to_device: HashMap<FixtureId, DeviceId>,
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

pub trait LEDDataOutput: ThreadedDevice {
    fn set_data_buffer(&mut self, receiver: Receiver<Vec<LED>>);
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

impl DeviceManager {
    pub fn new() -> DeviceManager {
        DeviceManager {
            devices: RwLock::new(HashMap::new()),
            led_data_buffers: RwLock::new(HashMap::new()),
            subscribed_events: SubscribedEvents {
                frame_resolver: vec![],
            },
            fixture_to_device: HashMap::new(),
        }
    }

    pub fn add_led_device(&self, id: DeviceId, mut device: Box<dyn LEDDataOutput>) {
        let (sender, receiver) = channel(10); // TODO: Get channel size from config
        device.set_data_buffer(receiver);

        let threaded_device = ThreadedDeviceWrapper::new(device);
        self.devices.write().insert(id.clone(), threaded_device);
        self.led_data_buffers.write().insert(id, sender);
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
            if let Some(sender) = self.led_data_buffers.read().get(device_id) {
                sender.send(event.data).ok();
            }
        }
    }
}
