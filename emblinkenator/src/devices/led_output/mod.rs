pub mod mqtt;
pub mod udp;

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, yield_now, JoinHandle},
};

use enum_dispatch::enum_dispatch;
use parking_lot::RwLock;
use tokio::sync::broadcast::Receiver;

use crate::frame_resolver::LEDFrame;

use self::{mqtt::MQTTSender, udp::UDPSender};

#[enum_dispatch]
pub trait LEDOutputDevice: Send + Sync {
    fn tick(&mut self);
    fn receive_data_from(&mut self, buffer: Receiver<LEDFrame>);
}

#[enum_dispatch(LEDOutputDevice)]
pub enum LEDDataOutputDeviceType {
    Mqtt(MQTTSender),
    Udp(UDPSender),
}

pub struct ThreadedLEDOutputDeviceWrapper {
    device: Arc<RwLock<LEDDataOutputDeviceType>>,
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl ThreadedLEDOutputDeviceWrapper {
    pub fn new(device: LEDDataOutputDeviceType) -> Self {
        let running: Arc<AtomicBool> = Arc::new(AtomicBool::default());
        running.store(true, Ordering::SeqCst);

        let alive = running.clone();

        let device = Arc::new(RwLock::new(device));
        let device_thread = Arc::clone(&device);

        let handle = Some(thread::spawn(move || {
            while alive.load(Ordering::SeqCst) {
                device_thread.write().tick();

                yield_now();
            }
        }));

        ThreadedLEDOutputDeviceWrapper {
            device,
            running,
            handle,
        }
    }

    pub async fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        self.handle
            .take()
            .expect("Called stop on non-running thread")
            .join()
            .expect("Could not join spawned thread");
    }

    pub fn receive_data_from(&mut self, buffer: Receiver<LEDFrame>) {
        self.device.write().receive_data_from(buffer)
    }
}
