use log::{error, warn};
use rumqttc::{AsyncClient, MqttOptions, QoS};
use tokio::{sync::broadcast::{Receiver, Sender, channel, error::TryRecvError}, task};
use serde::Deserialize;

use crate::{devices::{manager::{DeviceInput, DeviceInputType, DeviceOutput, DeviceOutputType}, threaded_device::{ThreadedDevice, ThreadedDeviceInputError, ThreadedDeviceOutputError}}, id::DeviceId};

#[derive(Clone, Deserialize)]
pub struct MQTTSenderConfig {
    name: String,
    host: String,
    port: u16,
    topic: String,
    credentials: Option<(String, String)>,
}

impl MQTTSenderConfig {
    pub fn new(
        name: String,
        host: String,
        port: u16,
        topic: String,
        credentials: Option<(String, String)>,
    ) -> Self {
        MQTTSenderConfig {
            name,
            host,
            port,
            topic,
            credentials,
        }
    }
}

pub struct MQTTSender {
    pub id: DeviceId,
    name: String,
    client: AsyncClient,
    topic: String,
    data_buffer_sender: Sender<DeviceInput>,
    data_buffer_receiver: Receiver<DeviceInput>
}

impl MQTTSender {
    pub fn new(id: DeviceId, config: MQTTSenderConfig) -> MQTTSender {
        let mut mqttoptions = MqttOptions::new(id.unprotect(), config.host, config.port);

        if let Some(credentials) = config.credentials {
            mqttoptions.set_credentials(credentials.0, credentials.1);
        }
        mqttoptions.set_keep_alive(10);

        let (client, mut eventloop) = AsyncClient::new(mqttoptions, 10);

        task::spawn(async move {
            loop {
                eventloop.poll().await.ok();
            }
        });

        let (sender, receiver) = channel(1);

        MQTTSender {
            id,
            name: config.name,
            client,
            topic: config.topic,
            data_buffer_sender: sender,
            data_buffer_receiver: receiver
        }
    }
}

impl ThreadedDevice for MQTTSender {
    fn run(&mut self) {
        match self.data_buffer_receiver.try_recv() {
            Err(err) => match err {
                TryRecvError::Lagged(missed) => warn!("MQTT device lagged by {} frames! (MQTT Device {})", missed, self.id.unprotect()),
                TryRecvError::Closed => error!("Data buffer exists but is closed! (MQTT Device {})", self.id.unprotect()),
                TryRecvError::Empty => {}
            },
            Ok(frame) => {
                let payload: Vec<u8> = frame.iter().flat_map(|l| l.flat_u8()).collect();
                pollster::block_on(self.client.publish(
                    self.topic.clone(),
                    QoS::ExactlyOnce,
                    true,
                    payload,
                ))
                .unwrap(); // TODO: This will panic and generally do bad things
            }
        }
    }

    fn get_inputs (&self) -> Vec<DeviceInputType> {
        vec![DeviceInputType::LEDData]
    }

    fn get_outputs (&self) -> Vec<DeviceOutputType> {
        vec![]
    }

    fn send_to_input (&self, index: usize) -> Result<Sender<DeviceInput>, ThreadedDeviceInputError> {
        Ok(self.data_buffer_sender.clone())
    }

    fn receive_output (&self, index: usize) -> Result<Receiver<DeviceOutput>, ThreadedDeviceOutputError> {
        Err(ThreadedDeviceOutputError::DoesNotExist)
    }
}
