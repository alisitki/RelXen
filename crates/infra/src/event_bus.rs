use tokio::sync::broadcast;

use relxen_app::{EventPublisher, OutboundEvent};

#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<OutboundEvent>,
}

impl EventBus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<OutboundEvent> {
        self.sender.subscribe()
    }
}

impl EventPublisher for EventBus {
    fn publish(&self, event: OutboundEvent) {
        let _ = self.sender.send(event);
    }
}
