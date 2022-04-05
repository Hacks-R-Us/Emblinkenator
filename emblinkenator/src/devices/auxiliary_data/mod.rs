pub mod noise;

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, yield_now, JoinHandle},
};

use enum_dispatch::enum_dispatch;
use parking_lot::RwLock;

use crate::{auxiliary_data::AuxiliaryDataType, frame::FrameData, id::AuxiliaryId};

use self::noise::NoiseAuxiliaryDataDevice;

#[derive(Debug, Clone)]
pub struct AuxDeviceData {
    pub aux_id: AuxiliaryId,
    pub data: AuxiliaryDataType,
}

#[enum_dispatch]
pub trait AuxiliaryDataDevice: Send + Sync {
    fn tick(&mut self);
    fn send_data_to_aux(&mut self, aux_id: AuxiliaryId);
    fn receive_next_frame_data_buffer(
        &mut self,
        buffer: tokio::sync::broadcast::Receiver<FrameData>,
    );
    fn send_into_buffer(&mut self, buffer: tokio::sync::broadcast::Sender<AuxDeviceData>);
}

#[enum_dispatch(AuxiliaryDataDevice)]
pub enum AuxiliaryDataDeviceType {
    Noise(NoiseAuxiliaryDataDevice),
}

pub struct ThreadedAuxiliaryDeviceWrapper {
    device: Arc<RwLock<AuxiliaryDataDeviceType>>,
    running: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl ThreadedAuxiliaryDeviceWrapper {
    pub fn new(device: AuxiliaryDataDeviceType) -> Self {
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

        ThreadedAuxiliaryDeviceWrapper {
            running,
            handle,
            device,
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

    pub fn send_data_to_aux(&mut self, aux_id: AuxiliaryId) {
        self.device.write().send_data_to_aux(aux_id);
    }

    pub fn receive_next_frame_data_buffer(
        &mut self,
        buffer: tokio::sync::broadcast::Receiver<FrameData>,
    ) {
        self.device.write().receive_next_frame_data_buffer(buffer)
    }

    pub fn send_into_buffer(&mut self, buffer: tokio::sync::broadcast::Sender<AuxDeviceData>) {
        self.device.write().send_into_buffer(buffer)
    }
}
