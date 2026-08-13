use super::DebugCaptureError;

pub struct PoisonRecoveryMutex<T> {
    inner: std::sync::Mutex<T>,
}

impl<T> PoisonRecoveryMutex<T> {
    pub fn new(value: T) -> Self {
        Self {
            inner: std::sync::Mutex::new(value),
        }
    }

    /// Lock the mutex, recovering from poison if needed
    pub fn lock(&self) -> Result<std::sync::MutexGuard<'_, T>, DebugCaptureError> {
        match self.inner.lock() {
            Ok(guard) => Ok(guard),
            Err(poisoned) => {
                // Recover from poison - the data may be in an inconsistent state
                // but for debug capture this is acceptable (we might lose events)
                Ok(poisoned.into_inner())
            }
        }
    }
}
