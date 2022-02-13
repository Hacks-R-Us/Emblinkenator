use std::collections::HashMap;

use log::{debug, warn};
use parking_lot::RwLock;
use tokio::time::{self, Instant, Interval};

use crate::state::{ThreadedObject, WantsDeviceState};

#[derive(Clone)]
pub struct FrameData {
    pub frame: u32,
    pub frame_rate: u32,
}

struct FrameStats {
    pub frame_start_time: Instant,
    pub target_frame_time: u128,
}

pub struct FrameTimeKeeper {
    frame_data_blocking_senders: RwLock<HashMap<String, crossbeam::channel::Sender<FrameData>>>,
    next_frame_data_blocking_senders: RwLock<HashMap<String, crossbeam::channel::Sender<FrameData>>>,
    frame_data_non_blocking_senders: RwLock<HashMap<String, tokio::sync::broadcast::Sender<FrameData>>>,
    next_frame_data_non_blocking_senders: RwLock<HashMap<String, tokio::sync::broadcast::Sender<FrameData>>>,
    frame_rate: u32,
    clock_frame: Interval,
    frame_data: FrameData,
    next_frame_data: FrameData,
    frame_stats: FrameStats,
    late_time: u128,
    frame_buffer_size: u128
}

impl FrameTimeKeeper {
    pub fn new (frame_rate: u32, frame_buffer_size: u128) -> Self {
        // TODO: Dodgy!
        let frame_time: u64 = u64::from(1000 / frame_rate);
        let clock_frame = time::interval(time::Duration::from_millis(frame_time));

        FrameTimeKeeper {
            frame_data_blocking_senders: RwLock::new(HashMap::new()),
            next_frame_data_blocking_senders: RwLock::new(HashMap::new()),
            frame_data_non_blocking_senders: RwLock::new(HashMap::new()),
            next_frame_data_non_blocking_senders: RwLock::new(HashMap::new()),
            frame_rate,
            clock_frame,
            frame_data: FrameData::new(0, frame_rate),
            next_frame_data: FrameData::new(1, frame_rate),
            frame_stats: FrameStats::new(u128::from(frame_time)),
            frame_buffer_size,
            late_time: 0
        }
    }

    pub fn send_frame_data_to_blocking (&self, receiver_id: String, buffer: crossbeam::channel::Sender<FrameData>) {
        self.frame_data_blocking_senders.write().insert(receiver_id, buffer);
    }

    pub fn send_frame_data_to_non_blocking (&self, receiver_id: String, buffer: tokio::sync::broadcast::Sender<FrameData>) {
        self.frame_data_non_blocking_senders.write().insert(receiver_id, buffer);
    }

    pub fn send_next_frame_data_to_blocking (&self, receiver_id: String, buffer: crossbeam::channel::Sender<FrameData>) {
        self.next_frame_data_blocking_senders.write().insert(receiver_id, buffer);
    }

    pub fn send_next_frame_data_to_non_blocking (&self, receiver_id: String, buffer: tokio::sync::broadcast::Sender<FrameData>) {
        self.next_frame_data_non_blocking_senders.write().insert(receiver_id, buffer);
    }
}

impl ThreadedObject for FrameTimeKeeper {
    fn tick(&mut self) {
        pollster::block_on(self.clock_frame.tick());

        let last_frame_start = self.frame_stats.frame_start_time;
        let target_frame_time = self.frame_stats.target_frame_time;
        let elapsed_time = Instant::now()
                .duration_since(last_frame_start)
                .as_millis();

        if elapsed_time > target_frame_time {
            self.late_time += elapsed_time - target_frame_time;
            debug!(
                "Frame late by {}ms (Took {}ms)",
                elapsed_time - target_frame_time,
                elapsed_time
            );
        } else if self.late_time > 0 {
            match self.late_time.checked_sub(target_frame_time - elapsed_time) {
                Some(val) => self.late_time = val,
                None => self.late_time = 0
            }
        }

        if self.late_time >= target_frame_time * self.frame_buffer_size {
            warn!("Running late by {}ms", self.late_time);
        }

        // TODO: This is where a change to framerate would happen
        self.frame_data = self.next_frame_data.clone();
        self.next_frame_data = FrameData::new(self.frame_data.frame + 1, self.frame_rate);

        self.frame_stats = FrameStats::new(target_frame_time);

        for (_, sender) in self.frame_data_blocking_senders.write().iter() {
            sender.send(self.frame_data.clone()).ok();
        }

        for (_, sender) in self.frame_data_non_blocking_senders.write().iter() {
            sender.send(self.frame_data.clone()).ok();
        }

        for (_, sender) in self.next_frame_data_blocking_senders.write().iter() {
            sender.send(self.next_frame_data.clone()).ok();
        }

        for (_, sender) in self.next_frame_data_non_blocking_senders.write().iter() {
            sender.send(self.next_frame_data.clone()).ok();
        }
    }
}

impl WantsDeviceState for FrameTimeKeeper {
    fn on_device_added(&mut self, state: &crate::state::EmblinkenatorState, device_id: crate::id::DeviceId) {
        if let Some(device) = state.get_device(&device_id) {
            match &mut *device.write() {
                crate::devices::manager::ThreadedDeviceType::LEDDataOutput(_) => {}, // Nothing to do
                crate::devices::manager::ThreadedDeviceType::AuxiliaryData(aux_device) => {
                    let (sender, receiver) = tokio::sync::broadcast::channel(1);
                    aux_device.receive_next_frame_data_buffer(receiver);
                    self.send_next_frame_data_to_non_blocking(device_id.unprotect(), sender);
                },
            }
        }
    }
}

impl FrameData {
    pub fn new (frame: u32, frame_rate: u32) -> Self {
        FrameData {
            frame,
            frame_rate
        }
    }
}

impl FrameStats {
    pub fn new (target_frame_time: u128) -> Self {
        FrameStats {
            frame_start_time: Instant::now(),
            target_frame_time
        }
    }
}
