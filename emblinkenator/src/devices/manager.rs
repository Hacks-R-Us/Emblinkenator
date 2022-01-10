use enum_dispatch::enum_dispatch;
use log::{debug, error};
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};

use serde::Deserialize;
use tokio::sync::broadcast::{error::TryRecvError, Receiver, Sender};

use crate::{
    auxiliary_data::{AuxiliaryDataType, AuxiliaryDataTypeConsumer},
    frame::FrameData,
    frame_resolver::FrameResolverDataEvent,
    id::{DeviceId, FixtureId},
    led::LED,
    state::{EmblinkenatorState, ThreadedObject},
};

use super::{
    auxiliary_data::{noise::{NoiseAuxiliaryConfig, NoiseAuxiliaryDataDevice}},
    led_output::{mqtt::{MQTTSenderConfig, MQTTSender}, udp::{UDPSenderConfig, UDPSender}},
    threaded_device::{ThreadedDeviceWrapper},
};

pub struct DeviceManager {
    devices: RwLock<HashMap<DeviceId, Arc<ThreadedDeviceWrapper>>>,
    led_data_buffers: RwLock<HashMap<DeviceId, Sender<Vec<LED>>>>,
    subscribed_events: SubscribedEvents,
    event_emitters: RwLock<Vec<crossbeam::channel::Sender<DeviceManagerEvent>>>,
    fixture_to_device: HashMap<FixtureId, DeviceId>,
}

#[enum_dispatch(ThreadedDevice)]
pub enum DeviceType {
    LEDDataOutput(LEDDataOutputDeviceType),
    Auxiliary(AuxiliaryDataDeviceType)
}

#[enum_dispatch(ThreadedDevice)]
pub enum LEDDataOutputDeviceType {
    MQTT(MQTTSender),
    UPD(UDPSender)
}

#[enum_dispatch(ThreadedDevice)]
pub enum AuxiliaryDataDeviceType {
    Noise(NoiseAuxiliaryDataDevice)
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

struct DeviceConfigWithId(DeviceId, DeviceConfigType);
struct LEDOutputConfigWithId(DeviceId, LEDOutputConfigType);
struct AuxiliaryDataConfigWithId(DeviceId, AuxiliaryDataConfigType);

pub enum DeviceOutputType {
    Auxiliary(AuxiliaryDataTypeConsumer),
}

pub enum DeviceInputType {
    LEDData,
}

pub enum DeviceOutput {
    Auxiliary(AuxiliaryDataType),
}

#[derive(Clone)]
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
        _state: &EmblinkenatorState,
    ) {
        // Match type
        // Build devices
        // Link up buffers using state
        let device: DeviceType = config.into();
        let device = ThreadedDeviceWrapper::new(Box::new(device));
        self.add_device(id, device)
    }

    pub fn get_device(&self, id: DeviceId) -> Option<Arc<ThreadedDeviceWrapper>> {
        self.devices.read().get(&id).map(Arc::clone)
    }

    pub fn add_device(&self, id: DeviceId, device: ThreadedDeviceWrapper) {
        let mut devices_lock = self.devices.write();
        if devices_lock.contains_key(&id) {

        }

        devices_lock.insert(id, Arc::new(device));
    }

    /*pub fn add_led_device(&self, id: DeviceId, mut device: Box<dyn LEDDataOutput>) {
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
    }*/

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

impl From<DeviceConfigType> for DeviceType {
    fn from(config: DeviceConfigType) -> Self {
        let device_with_id =  DeviceConfigWithId(DeviceId::new(), config);
        device_with_id.into()
    }
}

impl From<DeviceConfigWithId> for DeviceType {
    fn from(device_with_id: DeviceConfigWithId) -> Self {
        let device_id = device_with_id.0;
        let config = device_with_id.1;

        match config {
            DeviceConfigType::LEDDataOutput(led_device_config) => {
                let led_device_config_with_id = LEDOutputConfigWithId(device_id, led_device_config);
                DeviceType::LEDDataOutput(led_device_config_with_id.into())
            },
            DeviceConfigType::Auxiliary(auxiliary_device_config) => {
                let auxiliary_device_config_with_id = AuxiliaryDataConfigWithId(device_id, auxiliary_device_config);
                DeviceType::Auxiliary(auxiliary_device_config_with_id.into())
            },
        }
    }
}

impl From<LEDOutputConfigType> for LEDDataOutputDeviceType {
    fn from(led_device_config: LEDOutputConfigType) -> Self {
        let led_device_config_with_id = LEDOutputConfigWithId(DeviceId::new(), led_device_config);
        led_device_config_with_id.into()
    }
}

impl From<LEDOutputConfigWithId> for LEDDataOutputDeviceType {
    fn from(led_device_config_with_id: LEDOutputConfigWithId) -> Self {
        let device_id = led_device_config_with_id.0;
        let led_device_config = led_device_config_with_id.1;

        match led_device_config {
            LEDOutputConfigType::MQTT(mqtt_device_config) => {
                LEDDataOutputDeviceType::MQTT(MQTTSender::new(device_id, mqtt_device_config))
            },
            LEDOutputConfigType::UDP(udp_device_config) => {
                LEDDataOutputDeviceType::UPD(UDPSender::new(device_id, udp_device_config))
            },
        }
    }
}

impl From<AuxiliaryDataConfigType> for AuxiliaryDataDeviceType {
    fn from(auxiliary_device_config: AuxiliaryDataConfigType) -> Self {
        let auxiliary_device_with_id = AuxiliaryDataConfigWithId(DeviceId::new(), auxiliary_device_config);
        auxiliary_device_with_id.into()
    }
}

impl From<AuxiliaryDataConfigWithId> for AuxiliaryDataDeviceType {
    fn from(auxiliary_device_config_with_id: AuxiliaryDataConfigWithId) -> Self {
        let device_id = auxiliary_device_config_with_id.0;
        let auxiliary_device_config = auxiliary_device_config_with_id.1;

        match auxiliary_device_config {
            AuxiliaryDataConfigType::Noise(noise_auxiliary_config) => {
                AuxiliaryDataDeviceType::Noise(NoiseAuxiliaryDataDevice::new(device_id, noise_auxiliary_config))
            },
        }
    }
}
