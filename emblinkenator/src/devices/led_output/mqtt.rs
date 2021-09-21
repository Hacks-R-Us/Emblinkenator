use log::{error, warn};
use rumqttc::{AsyncClient, MqttOptions, QoS};
use tokio::{sync::broadcast::{Receiver, error::TryRecvError}, task};
use serde::Deserialize;

use crate::{devices::threaded_device::ThreadedDevice, id::DeviceId, led::LED};

use super::LEDDataOutput;

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
    data_buffer: Option<Receiver<Vec<LED>>>
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

        MQTTSender {
            id,
            name: config.name,
            client,
            topic: config.topic,
            data_buffer: None
        }
    }
}

impl LEDDataOutput for MQTTSender {
    fn set_data_buffer(&mut self, receiver: Receiver<Vec<LED>>) {
        self.data_buffer.replace(receiver);
    }
}

impl ThreadedDevice for MQTTSender {
    fn run(&mut self) {
        if let Some(buffer) = &mut self.data_buffer {
            match buffer.try_recv() {
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
    }
}
