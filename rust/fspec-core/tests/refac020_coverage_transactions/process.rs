use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use fs2::FileExt;
use serde_json::{json, Value};

use crate::support::*;

pub struct Worker {
    child: Child,
    ipc: PathBuf,
}

impl Worker {
    pub fn start(root: &Path, label: &str, command: &str, args: Value) -> TestResult<Self> {
        let ipc = root.join("ipc").join(label);
        std::fs::create_dir_all(&ipc)?;
        let child = Command::new(std::env::current_exe()?)
            .args(["--ignored", "--exact", "worker::transaction_worker_process", "--nocapture"])
            .env("REFAC020_WORKER_ROOT", root)
            .env("REFAC020_WORKER_IPC", &ipc)
            .env("REFAC020_WORKER_COMMAND", command)
            .env("REFAC020_WORKER_ARGS", args.to_string())
            .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
            .spawn()?;
        let mut worker = Self { child, ipc };
        worker.await_file("ready")?;
        Ok(worker)
    }

    pub fn begin(&mut self) -> TestResult {
        std::fs::write(self.ipc.join("go"), b"go")?;
        self.await_file("entered")
    }

    fn await_file(&mut self, name: &str) -> TestResult {
        let deadline = Instant::now() + Duration::from_secs(15);
        while !self.ipc.join(name).exists() {
            if let Some(status) = self.child.try_wait()? {
                return Err(format!("worker exited {status} before {name}").into());
            }
            if Instant::now() >= deadline {
                return Err(format!("worker never reached {name}").into());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }

    pub fn assert_pending(&mut self) -> TestResult {
        assert!(!self.ipc.join("result.json").exists(), "writer completed while another process held its sidecar transaction");
        assert!(self.child.try_wait()?.is_none(), "writer exited while transaction was held");
        Ok(())
    }

    pub fn assert_waiting(&mut self) -> TestResult {
        let deadline = Instant::now() + Duration::from_millis(500);
        loop {
            self.assert_pending()?;
            if Instant::now() >= deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }

    pub fn finish(mut self) -> TestResult<Value> {
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            if let Some(status) = self.child.try_wait()? {
                assert!(status.success(), "worker harness failed: {status}");
                return Ok(serde_json::from_slice(&std::fs::read(self.ipc.join("result.json"))?)?);
            }
            if Instant::now() >= deadline {
                return Err("writer did not complete after lock release".into());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub fn hold_transaction(root: &Path) -> TestResult<File> {
    let lock = OpenOptions::new().create(true).truncate(false).read(true).write(true)
        .open(root.join(format!("{SIDECAR}.lock")))?;
    lock.lock_exclusive()?;
    Ok(lock)
}

pub fn commit_while_locked(root: &Path) -> TestResult<Value> {
    let mut latest = load(root)?;
    latest["scenarios"][0]["unknownScenario"]["committedWhileWaiting"] = json!(true);
    latest["scenarios"][0]["testMappings"].as_array_mut().ok_or("mapping array")?
        .push(mapping("committed-while-locked"));
    save(root, &latest)?;
    Ok(latest)
}
