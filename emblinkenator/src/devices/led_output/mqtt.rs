use std::{
    thread::{self, sleep, yield_now},
    time::Duration,
};

use log::{error, warn};
use rumqttc::{Client, MqttOptions, QoS};
use serde::Deserialize;
use tokio::sync::broadcast::{error::TryRecvError, Receiver};

use crate::{frame_resolver::LEDFrame, id::DeviceId};

use super::LEDOutputDevice;

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
    client: Client,
    topic: String,
    data_buffer_receiver: Option<Receiver<LEDFrame>>,
}

impl MQTTSender {
    pub fn new(id: DeviceId, config: MQTTSenderConfig) -> MQTTSender {
        let mut mqttoptions = MqttOptions::new(id.unprotect(), config.host, config.port);

        if let Some(credentials) = config.credentials {
            mqttoptions.set_credentials(credentials.0, credentials.1);
        }
        mqttoptions.set_keep_alive(Duration::from_secs(10));

        let (client, mut connection) = Client::new(mqttoptions, 10);

        thread::spawn(move || {
            loop {
                // Poll for the sake of making progress, but we don't care about the result
                for (_, _) in connection.iter().enumerate() {
                    yield_now();
                }
            }
        });

        MQTTSender {
            id,
            name: config.name,
            client,
            topic: config.topic,
            data_buffer_receiver: None,
        }
    }
}

impl LEDOutputDevice for MQTTSender {
    fn tick(&mut self) {
        if let Some(data_buffer_receiver) = &mut self.data_buffer_receiver {
            match data_buffer_receiver.try_recv() {
                Err(err) => match err {
                    TryRecvError::Lagged(missed) => warn!(
                        "MQTT device lagged by {} frames! (MQTT Device {})",
                        missed,
                        self.id.unprotect()
                    ),
                    TryRecvError::Closed => error!(
                        "Data buffer exists but is closed! (MQTT Device {})",
                        self.id.unprotect()
                    ), // TODO: Remove buffer
                    TryRecvError::Empty => {}
                },
                Ok(frame) => {
                    let payload: Vec<u8> = frame.iter().flat_map(|l| l.flat_u8()).collect();
                    self.client
                        .publish(self.topic.clone(), QoS::ExactlyOnce, true, payload)
                        .unwrap(); // TODO: This will panic and generally do bad things
                }
            }
        }
    }

    fn receive_data_from(&mut self, buffer: Receiver<crate::frame_resolver::LEDFrame>) {
        self.data_buffer_receiver.replace(buffer);
    }
}
