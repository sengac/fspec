//! Input queue for queuing user input during agent execution
//!
//! Uses tokio::sync::mpsc for async-native queuing.

use tokio::sync::mpsc;

/// Input queue using tokio channels
pub struct InputQueue {
    tx: mpsc::UnboundedSender<String>,
    rx: mpsc::UnboundedReceiver<String>,
}

impl InputQueue {
    /// Create new input queue
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self { tx, rx }
    }

    /// Queue an input
    pub fn queue_input(&self, input: String) -> Result<(), mpsc::error::SendError<String>> {
        self.tx.send(input)
    }

    /// Try to dequeue an input (non-blocking)
    pub fn try_dequeue(&mut self) -> Option<String> {
        self.rx.try_recv().ok()
    }

    /// Dequeue all inputs
    pub fn dequeue_all(&mut self) -> Vec<String> {
        let mut inputs = Vec::new();
        while let Some(input) = self.try_dequeue() {
            inputs.push(input);
        }
        inputs
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        self.rx.is_empty()
    }
}

impl Default for InputQueue {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn test_input_queue() {
        let mut queue = InputQueue::new();

        queue.queue_input("first".to_string()).unwrap();
        queue.queue_input("second".to_string()).unwrap();

        let all = queue.dequeue_all();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0], "first");
        assert_eq!(all[1], "second");
    }
}
