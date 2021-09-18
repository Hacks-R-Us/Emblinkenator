use std::{sync::{Arc, atomic::{AtomicBool, Ordering}}, thread::{self, JoinHandle}};

pub struct ThreadedDeviceWrapper {
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

pub trait ThreadedDevice: Send {
    fn run (&mut self);
}

impl ThreadedDeviceWrapper {
    pub fn new(mut device: Box<dyn ThreadedDevice>) -> Self {
        let running: Arc<AtomicBool> = Arc::new(AtomicBool::default());
        running.store(true, Ordering::SeqCst);

        let alive = running.clone();

        let handle = Some(thread::spawn(move || {
            while alive.load(Ordering::SeqCst) {
                device.run()
            }
        }));

        ThreadedDeviceWrapper {
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
