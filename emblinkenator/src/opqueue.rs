pub struct Opqueue<T> {
    opqueue: Vec<OpqueueEntry<T>>,
}

#[derive(Clone)]
struct OpqueueEntry<T> {
    pub target_frame: u32,
    pub entry: T,
}

impl<T> Opqueue<T> {
    pub fn new() -> Self {
        Opqueue { opqueue: vec![] }
    }

    pub fn add_op(&mut self, op: T, target_frame: u32) {
        self.opqueue.push(OpqueueEntry::new(op, target_frame));
    }

    pub fn get_frame_ops(&mut self) -> Vec<T> {
        self.opqueue
            .iter_mut()
            .for_each(|mut f| f.target_frame -= 1);
        let active: Vec<T> = self
            .opqueue
            .drain_filter(|f| f.target_frame == 0)
            .map(|e| e.entry)
            .collect();

        active
    }
}

impl<T> OpqueueEntry<T> {
    fn new(op: T, target_frame: u32) -> Self {
        OpqueueEntry {
            target_frame,
            entry: op,
        }
    }
}
