use tokio::sync::broadcast::Receiver;

pub trait EventEmitter<T> {
    fn subscribe(&self) -> Receiver<T>;
}
