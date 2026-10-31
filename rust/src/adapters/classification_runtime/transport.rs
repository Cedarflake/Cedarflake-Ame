use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use windows_sys::Win32::System::Threading::{BELOW_NORMAL_PRIORITY_CLASS, CREATE_NO_WINDOW};

use super::RuntimeError;
use super::protocol::{self, Frame, Request, Response};
use super::windows_job::WorkerJob;

pub(super) struct Transport {
    child: Child,
    job: WorkerJob,
    requests: Option<SyncSender<Frame<Request>>>,
    responses: Receiver<Result<Frame<Response>, RuntimeError>>,
    io_thread: Option<JoinHandle<()>>,
    retired: bool,
}

impl Transport {
    pub fn start(executable: &Path) -> Result<Self, RuntimeError> {
        if !executable.is_absolute() {
            return Err(RuntimeError::Configuration);
        }
        Self::launch(Command::new(executable))
    }

    fn launch(mut command: Command) -> Result<Self, RuntimeError> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .env("ORT_DISABLE_TELEMETRY", "1")
            .env("TOKENIZERS_PARALLELISM", "false")
            .creation_flags(CREATE_NO_WINDOW | BELOW_NORMAL_PRIORITY_CLASS)
            .spawn()?;
        let job = match WorkerJob::attach(&child) {
            Ok(job) => job,
            Err(error) => {
                let _ = child.kill();
                child.wait()?;
                return Err(error.into());
            }
        };
        let mut input = child.stdin.take().expect("the worker has a piped stdin");
        let mut output = child.stdout.take().expect("the worker has a piped stdout");
        let (requests, incoming) = mpsc::sync_channel::<Frame<Request>>(1);
        let (outgoing, responses) = mpsc::sync_channel(1);
        let io_thread = thread::Builder::new()
            .name("ame-classification-pipe".into())
            .spawn(move || {
                while let Ok(frame) = incoming.recv() {
                    let result = protocol::write(&mut input, &frame.message, &frame.payload)
                        .and_then(|()| protocol::read(&mut output))
                        .and_then(|response| response.ok_or(RuntimeError::Exited));
                    let failed = result.is_err();
                    if outgoing.send(result).is_err() || failed {
                        break;
                    }
                }
            });
        let io_thread = match io_thread {
            Ok(thread) => thread,
            Err(error) => {
                let _ = job.terminate();
                child.wait()?;
                return Err(error.into());
            }
        };
        Ok(Self {
            child,
            job,
            requests: Some(requests),
            responses,
            io_thread: Some(io_thread),
            retired: false,
        })
    }

    pub fn request(
        &mut self,
        frame: Frame<Request>,
        timeout: Duration,
        cancelled: &AtomicBool,
    ) -> Result<Frame<Response>, RuntimeError> {
        if self.retired {
            return Err(RuntimeError::Exited);
        }
        if cancelled.load(Ordering::Acquire) {
            return Err(RuntimeError::Cancelled);
        }
        self.requests
            .as_ref()
            .ok_or(RuntimeError::Exited)?
            .try_send(frame)
            .map_err(|_| RuntimeError::Protocol)?;
        let deadline = Instant::now() + timeout;
        loop {
            if cancelled.load(Ordering::Acquire) {
                return Err(RuntimeError::Cancelled);
            }
            if Instant::now() >= deadline {
                return Err(RuntimeError::TimedOut);
            }
            match self.responses.recv_timeout(Duration::from_millis(10)) {
                Ok(result) => return result,
                Err(mpsc::RecvTimeoutError::Disconnected) => return Err(RuntimeError::Exited),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    }

    pub fn retire(&mut self) -> Result<(), RuntimeError> {
        if self.retired {
            return Ok(());
        }
        self.requests.take();
        if self.child.try_wait()?.is_none() {
            self.job.terminate()?;
        }
        self.child.wait()?;
        if let Some(thread) = self.io_thread.take() {
            thread.join().map_err(|_| RuntimeError::Protocol)?;
        }
        self.retired = true;
        Ok(())
    }

    pub fn process_id(&self) -> u32 {
        self.child.id()
    }

    pub fn is_retired(&self) -> bool {
        self.retired
    }
}

#[cfg(test)]
mod tests;

impl Drop for Transport {
    fn drop(&mut self) {
        let _ = self.retire();
    }
}
