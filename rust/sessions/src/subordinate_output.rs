use codelet_rpc_types::StreamChunk;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, Weak};
use tokio::sync::broadcast;

#[derive(Debug, thiserror::Error)]
pub enum RelayOutputError {
    #[error("subordinate output journal I/O failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("subordinate output journal serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("subordinate output journal lock poisoned")]
    Poisoned,
    #[error("subordinate output journal publication failed: {0}")]
    Publication(String),
}

struct Journal {
    file: File,
    base: u64,
    end: u64,
    failure: Option<String>,
    cursors: Vec<Weak<AtomicU64>>,
}

#[derive(Default)]
pub(crate) struct RelayOutput {
    journal: Mutex<Weak<Mutex<Journal>>>,
}

pub struct RelaySubscription {
    journal: Arc<Mutex<Journal>>,
    notifications: broadcast::Receiver<StreamChunk>,
    cursor: Arc<AtomicU64>,
}

impl RelayOutput {
    pub(crate) fn subscribe(
        &self,
        notifications: broadcast::Receiver<StreamChunk>,
    ) -> Result<RelaySubscription, RelayOutputError> {
        let mut slot = self
            .journal
            .lock()
            .map_err(|_| RelayOutputError::Poisoned)?;
        let journal = match slot.upgrade() {
            Some(journal) => journal,
            None => {
                let journal = Arc::new(Mutex::new(Journal {
                    file: tempfile::tempfile()?,
                    base: 0,
                    end: 0,
                    failure: None,
                    cursors: Vec::new(),
                }));
                *slot = Arc::downgrade(&journal);
                journal
            }
        };
        let cursor = {
            let mut journal = journal.lock().map_err(|_| RelayOutputError::Poisoned)?;
            let cursor = Arc::new(AtomicU64::new(journal.end));
            journal.cursors.retain(|cursor| cursor.strong_count() > 0);
            journal.cursors.push(Arc::downgrade(&cursor));
            cursor
        };
        Ok(RelaySubscription {
            journal,
            notifications,
            cursor,
        })
    }

    pub(crate) fn append(&self, chunk: &StreamChunk) {
        if let Err(error) = self.try_append(chunk) {
            tracing::error!(%error, "BUG-217: subordinate output publication failed");
        }
    }

    fn try_append(&self, chunk: &StreamChunk) -> Result<(), RelayOutputError> {
        let slot = self
            .journal
            .lock()
            .map_err(|_| RelayOutputError::Poisoned)?;
        let Some(journal) = slot.upgrade() else {
            return Ok(());
        };
        let mut journal = journal.lock().map_err(|_| RelayOutputError::Poisoned)?;
        if journal.failure.is_some() {
            return Ok(());
        }
        let result = journal.append(chunk);
        if let Err(error) = &result {
            journal.failure = Some(error.to_string());
        }
        result
    }
}

impl Journal {
    fn reclaim(&mut self) -> Result<(), RelayOutputError> {
        self.cursors.retain(|cursor| cursor.strong_count() > 0);
        let minimum = self
            .cursors
            .iter()
            .filter_map(Weak::upgrade)
            .map(|cursor| cursor.load(Ordering::Acquire))
            .min()
            .unwrap_or(self.end);
        let consumed = minimum - self.base;
        let retained = self.end - minimum;
        if retained == 0 {
            self.file.set_len(0)?;
            self.base = self.end;
        } else if consumed >= 64 * 1024 && consumed >= retained {
            self.file.seek(SeekFrom::Start(consumed))?;
            let mut replacement = tempfile::tempfile()?;
            std::io::copy(&mut (&mut self.file).take(retained), &mut replacement)?;
            self.file = replacement;
            self.base = minimum;
        }
        Ok(())
    }

    fn append(&mut self, chunk: &StreamChunk) -> Result<(), RelayOutputError> {
        self.reclaim()?;
        self.file.seek(SeekFrom::Start(self.end - self.base))?;
        serde_json::to_writer(&mut self.file, chunk)?;
        self.file.write_all(b"\n")?;
        self.end = self.base + self.file.stream_position()?;
        Ok(())
    }
}

impl RelaySubscription {
    pub(crate) fn fork(&self) -> Result<Self, RelayOutputError> {
        let mut journal = self
            .journal
            .lock()
            .map_err(|_| RelayOutputError::Poisoned)?;
        let cursor = Arc::new(AtomicU64::new(self.cursor.load(Ordering::Acquire)));
        journal.cursors.retain(|cursor| cursor.strong_count() > 0);
        journal.cursors.push(Arc::downgrade(&cursor));
        Ok(Self {
            journal: self.journal.clone(),
            notifications: self.notifications.resubscribe(),
            cursor,
        })
    }

    fn read_next(&mut self) -> Result<Option<StreamChunk>, RelayOutputError> {
        let mut journal = self
            .journal
            .lock()
            .map_err(|_| RelayOutputError::Poisoned)?;
        let cursor = self.cursor.load(Ordering::Acquire);
        if cursor == journal.end {
            return match &journal.failure {
                Some(failure) => Err(RelayOutputError::Publication(failure.clone())),
                None => Ok(None),
            };
        }
        let offset = cursor - journal.base;
        journal.file.seek(SeekFrom::Start(offset))?;
        let mut record = String::new();
        let consumed = BufReader::new(&mut journal.file).read_line(&mut record)?;
        let chunk = serde_json::from_str(&record)?;
        self.cursor
            .store(cursor + consumed as u64, Ordering::Release);
        Ok(Some(chunk))
    }

    pub async fn recv(&mut self) -> Result<Option<StreamChunk>, RelayOutputError> {
        loop {
            if let Some(chunk) = self.read_next()? {
                return Ok(Some(chunk));
            }
            match self.notifications.recv().await {
                Ok(_) | Err(broadcast::error::RecvError::Lagged(_)) => {}
                Err(broadcast::error::RecvError::Closed) => return self.read_next(),
            }
        }
    }
}
