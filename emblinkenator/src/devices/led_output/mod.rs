pub mod mqtt;
pub mod udp;

use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, thread::{JoinHandle, self, sleep}, time::Duration};

use enum_dispatch::enum_dispatch;

use self::{mqtt::MQTTSender, udp::UDPSender};

#[enum_dispatch]
pub trait LEDOutputDevice: Send + Sync {
    fn tick (&mut self);
}

#[enum_dispatch(LEDOutputDevice)]
pub enum LEDDataOutputDeviceType {
    MQTT(MQTTSender),
    UPD(UDPSender)
}

pub struct ThreadedLEDOutputDeviceWrapper {
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl ThreadedLEDOutputDeviceWrapper {
    pub fn new(mut device: LEDDataOutputDeviceType) -> Self {
        let running: Arc<AtomicBool> = Arc::new(AtomicBool::default());
        running.store(true, Ordering::SeqCst);

        let alive = running.clone();

        let handle = Some(thread::spawn(move || {
            while alive.load(Ordering::SeqCst) {
                device.tick();

                sleep(Duration::from_millis(1));
            }
        }));

        ThreadedLEDOutputDeviceWrapper {
            running,
            handle,
        }
    }

    pub async fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        self.handle
            .take().expect("Called stop on non-running thread")
            .join().expect("Could not join spawned thread");
    }
}