use protected_id::ProtectedId;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use tokio::task;
use serde::Deserialize;

use crate::{id::DeviceId, led::LED};

use super::manager::LEDDataOutput;

// TODO: All of this is temporary.

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
        }
    }
}

impl LEDDataOutput for MQTTSender {
    fn on_frame(&self, frame: Vec<LED>) {
        let payload: Vec<u8> = frame.iter().flat_map(|l| l.flat_u8()).collect();
        pollster::block_on(self.client.publish(
            self.topic.clone(),
            QoS::ExactlyOnce,
            true,
            payload,
        ))
        .unwrap(); // TODO: This will panic, block, generally do bad things
    }
}
