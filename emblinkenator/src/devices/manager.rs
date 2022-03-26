use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};

use serde::Deserialize;
use tokio::sync::broadcast::Sender;

use crate::{
    auxiliary_data::{AuxiliaryDataType, AuxiliaryDataTypeConsumer},
    id::DeviceId,
    led::LED,
    state::ThreadedObject,
};

use super::{
    auxiliary_data::{
        noise::{NoiseAuxiliaryConfig, NoiseAuxiliaryDataDevice},
        AuxiliaryDataDeviceType, ThreadedAuxiliaryDeviceWrapper,
    },
    led_output::{
        mqtt::{MQTTSender, MQTTSenderConfig},
        udp::{UDPSender, UDPSenderConfig},
        LEDDataOutputDeviceType, ThreadedLEDOutputDeviceWrapper,
    },
};

pub struct DeviceManager {
    devices: RwLock<HashMap<DeviceId, Arc<RwLock<ThreadedDeviceType>>>>,
    led_data_buffers: RwLock<HashMap<DeviceId, Sender<Vec<LED>>>>,
    event_emitters: RwLock<Vec<crossbeam::channel::Sender<DeviceManagerEvent>>>,
}

pub enum ThreadedDeviceType {
    LEDDataOutput(ThreadedLEDOutputDeviceWrapper),
    AuxiliaryData(ThreadedAuxiliaryDeviceWrapper),
}

pub enum DeviceType {
    LEDDataOutput(LEDDataOutputDeviceType),
    Auxiliary(AuxiliaryDataDeviceType),
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

impl DeviceManager {
    pub fn new() -> DeviceManager {
        DeviceManager {
            devices: RwLock::new(HashMap::new()),
            led_data_buffers: RwLock::new(HashMap::new()),
            event_emitters: RwLock::new(vec![]),
        }
    }

    pub fn add_device_from_config(&mut self, id: DeviceId, config: DeviceConfigType) {
        let device: DeviceType = config.into();
        let device = match device {
            DeviceType::LEDDataOutput(led_data_output) => ThreadedDeviceType::LEDDataOutput(
                ThreadedLEDOutputDeviceWrapper::new(led_data_output),
            ),
            DeviceType::Auxiliary(auxiliary) => {
                ThreadedDeviceType::AuxiliaryData(ThreadedAuxiliaryDeviceWrapper::new(auxiliary))
            }
        };
        self.add_device(id, device)
    }

    pub fn get_device(&self, id: &DeviceId) -> Option<Arc<RwLock<ThreadedDeviceType>>> {
        self.devices.read().get(id).map(Arc::clone)
    }

    pub fn add_device(&self, id: DeviceId, device: ThreadedDeviceType) {
        let mut devices_lock = self.devices.write();
        if devices_lock.contains_key(&id) {
            todo!()
        }

        devices_lock.insert(id.clone(), Arc::new(RwLock::new(device)));

        self.emit_device_added(id);
    }

    pub fn remove_device(&self, _id: DeviceId) {
        todo!()
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
    fn tick(&mut self) {}
}

impl From<DeviceConfigType> for DeviceType {
    fn from(config: DeviceConfigType) -> Self {
        let device_with_id = DeviceConfigWithId(DeviceId::new(), config);
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
            }
            DeviceConfigType::Auxiliary(auxiliary_device_config) => {
                let auxiliary_device_config_with_id =
                    AuxiliaryDataConfigWithId(device_id, auxiliary_device_config);
                DeviceType::Auxiliary(auxiliary_device_config_with_id.into())
            }
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
                LEDDataOutputDeviceType::Mqtt(MQTTSender::new(device_id, mqtt_device_config))
            }
            LEDOutputConfigType::UDP(udp_device_config) => {
                LEDDataOutputDeviceType::Udp(UDPSender::new(device_id, udp_device_config))
            }
        }
    }
}

impl From<AuxiliaryDataConfigType> for AuxiliaryDataDeviceType {
    fn from(auxiliary_device_config: AuxiliaryDataConfigType) -> Self {
        let auxiliary_device_with_id =
            AuxiliaryDataConfigWithId(DeviceId::new(), auxiliary_device_config);
        auxiliary_device_with_id.into()
    }
}

impl From<AuxiliaryDataConfigWithId> for AuxiliaryDataDeviceType {
    fn from(auxiliary_device_config_with_id: AuxiliaryDataConfigWithId) -> Self {
        let device_id = auxiliary_device_config_with_id.0;
        let auxiliary_device_config = auxiliary_device_config_with_id.1;

        match auxiliary_device_config {
            AuxiliaryDataConfigType::Noise(noise_auxiliary_config) => {
                AuxiliaryDataDeviceType::Noise(NoiseAuxiliaryDataDevice::new(
                    device_id,
                    noise_auxiliary_config,
                ))
            }
        }
    }
}
