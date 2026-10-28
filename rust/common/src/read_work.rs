use std::cell::RefCell;
use std::fs::{File, TryLockError};
use std::io::{self, BufRead, Read};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex, MutexGuard, TryLockError as MutexTryLockError,
};
use std::time::Duration;

type Observer = Arc<dyn Fn(&'static str) + Send + Sync>;

#[derive(Clone, Default)]
pub struct ReadCancellation(Arc<AtomicBool>);

#[derive(Default)]
struct Context {
    cancellation: Option<ReadCancellation>,
    observer: Option<Observer>,
    checkpoints: usize,
}

fn active() -> bool {
    CONTEXT.with(|context| context.borrow().cancellation.is_some())
}

pub fn read_file(path: &std::path::Path, boundary: &'static str) -> io::Result<Vec<u8>> {
    checkpoint(boundary)?;
    let mut content = Vec::new();
    read_to_end(&mut File::open(path)?, &mut content, boundary)?;
    Ok(content)
}

pub fn read_until(
    reader: &mut impl BufRead,
    delimiter: u8,
    content: &mut Vec<u8>,
) -> io::Result<usize> {
    let start = content.len();
    loop {
        checkpoint("history.scan.bytes")?;
        let buffer = match reader.fill_buf() {
            Ok(buffer) => buffer,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if buffer.is_empty() {
            break;
        }
        let end = buffer.iter().position(|byte| *byte == delimiter);
        let count = end.map_or(buffer.len(), |position| position + 1);
        content.extend_from_slice(&buffer[..count]);
        reader.consume(count);
        if end.is_some() {
            break;
        }
    }
    Ok(content.len() - start)
}

thread_local! {
    static CONTEXT: RefCell<Context> = const { RefCell::new(Context {
        cancellation: None, observer: None, checkpoints: 0,
    }) };
}

impl ReadCancellation {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn run<T>(&self, operation: impl FnOnce() -> T) -> T {
        struct Restore(Option<ReadCancellation>);
        impl Drop for Restore {
            fn drop(&mut self) {
                CONTEXT.with(|context| context.borrow_mut().cancellation = self.0.take());
            }
        }
        let _restore = Restore(
            CONTEXT.with(|context| context.borrow_mut().cancellation.replace(self.clone())),
        );
        operation()
    }
}

pub fn checkpoint(boundary: &'static str) -> io::Result<()> {
    let (cancellation, observer) = CONTEXT.with(|context| {
        let mut context = context.borrow_mut();
        context.checkpoints += 1;
        (context.cancellation.clone(), context.observer.clone())
    });
    let check = || {
        if cancellation
            .as_ref()
            .is_some_and(|token| token.0.load(Ordering::Acquire))
        {
            Err(io::Error::new(
                io::ErrorKind::Interrupted,
                "history read cancelled",
            ))
        } else {
            Ok(())
        }
    };
    check()?;
    if let Some(observer) = observer {
        observer(boundary);
    }
    check()
}

#[doc(hidden)]
pub fn observe<T>(observer: Observer, operation: impl FnOnce() -> T) -> (T, usize) {
    struct Restore(Option<Observer>, usize);
    impl Drop for Restore {
        fn drop(&mut self) {
            CONTEXT.with(|context| {
                let mut context = context.borrow_mut();
                context.observer = self.0.take();
                context.checkpoints = self.1;
            });
        }
    }
    let _restore = CONTEXT.with(|context| {
        let mut context = context.borrow_mut();
        let previous = Restore(context.observer.replace(observer), context.checkpoints);
        context.checkpoints = 0;
        previous
    });
    let result = operation();
    (result, CONTEXT.with(|context| context.borrow().checkpoints))
}

pub fn lock<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>, String> {
    if !active() {
        return mutex.lock().map_err(|error| error.to_string());
    }
    loop {
        checkpoint("read.mutex").map_err(|error| error.to_string())?;
        match mutex.try_lock() {
            Ok(guard) => return Ok(guard),
            Err(MutexTryLockError::Poisoned(error)) => return Err(error.to_string()),
            Err(MutexTryLockError::WouldBlock) => std::thread::sleep(Duration::from_millis(5)),
        }
    }
}

pub fn lock_file(file: &File) -> io::Result<()> {
    if !active() {
        return file.lock();
    }
    loop {
        checkpoint("read.lease")?;
        match file.try_lock() {
            Ok(()) => return Ok(()),
            Err(TryLockError::Error(error)) => return Err(error),
            Err(TryLockError::WouldBlock) => std::thread::sleep(Duration::from_millis(5)),
        }
    }
}

pub fn read_to_end(
    reader: &mut impl Read,
    content: &mut Vec<u8>,
    boundary: &'static str,
) -> io::Result<()> {
    let mut buffer = [0; 64 * 1024];
    loop {
        checkpoint(boundary)?;
        let count = match reader.read(&mut buffer) {
            Ok(count) => count,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if count == 0 {
            return Ok(());
        }
        content.extend_from_slice(&buffer[..count]);
    }
}
