use std::os::unix::net::UnixStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SyncSender, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use crate::error::{Error, Result};

pub const MAX_ACTIVE_HANDLERS: usize = 16;

pub trait ConnectionService: Send + Sync + 'static {
    fn handle(&self, stream: UnixStream);
}

struct Slot {
    sender: SyncSender<UnixStream>,
    busy: Arc<AtomicBool>,
}

pub struct WorkerPool {
    slots: Vec<Slot>,
    threads: Vec<JoinHandle<()>>,
    cursor: usize,
}

impl WorkerPool {
    pub fn start<S>(service: &Arc<S>) -> Result<Self>
    where
        S: ConnectionService,
    {
        let mut slots = Vec::with_capacity(MAX_ACTIVE_HANDLERS);
        let mut threads = Vec::with_capacity(MAX_ACTIVE_HANDLERS);
        for index in 0..MAX_ACTIVE_HANDLERS {
            let (sender, receiver) = sync_channel::<UnixStream>(1);
            let busy = Arc::new(AtomicBool::new(false));
            let worker_busy = Arc::clone(&busy);
            let worker_service = Arc::clone(service);
            let thread = thread::Builder::new()
                .name(format!("weyriva-ipc-{index}"))
                .spawn(move || {
                    while let Ok(stream) = receiver.recv() {
                        let guard = BusyGuard(Arc::clone(&worker_busy));
                        worker_service.handle(stream);
                        drop(guard);
                    }
                })
                .map_err(|error| Error::io("cannot start IPC handler", &error))?;
            slots.push(Slot { sender, busy });
            threads.push(thread);
        }
        Ok(Self {
            slots,
            threads,
            cursor: 0,
        })
    }

    pub fn dispatch(&mut self, stream: UnixStream) -> std::result::Result<(), UnixStream> {
        let mut pending = stream;
        for offset in 0..self.slots.len() {
            let index = (self.cursor + offset) % self.slots.len();
            let slot = &self.slots[index];
            if slot
                .busy
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                continue;
            }
            match slot.sender.send(pending) {
                Ok(()) => {
                    self.cursor = (index + 1) % self.slots.len();
                    return Ok(());
                }
                Err(error) => {
                    slot.busy.store(false, Ordering::Release);
                    pending = error.0;
                }
            }
        }
        Err(pending)
    }

    pub fn shutdown(mut self) -> Result<()> {
        self.slots.clear();
        let mut failed = false;
        for thread in self.threads.drain(..) {
            failed |= thread.join().is_err();
        }
        if failed {
            Err(Error::new("ipc_worker_failed", "an IPC handler panicked"))
        } else {
            Ok(())
        }
    }
}

struct BusyGuard(Arc<AtomicBool>);

impl Drop for BusyGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

pub struct LockedService<T> {
    inner: Mutex<T>,
}

impl<T> LockedService<T> {
    pub const fn new(inner: T) -> Self {
        Self {
            inner: Mutex::new(inner),
        }
    }

    pub fn lock(&self) -> Result<std::sync::MutexGuard<'_, T>> {
        self.inner
            .lock()
            .map_err(|_| Error::new("ipc_worker_failed", "IPC service lock is poisoned"))
    }
}

#[cfg(test)]
mod tests {
    use std::net::Shutdown;
    use std::os::unix::net::UnixStream;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    use super::{ConnectionService, MAX_ACTIVE_HANDLERS, WorkerPool};

    struct BlockingService {
        active: AtomicUsize,
        release: AtomicBool,
    }

    impl ConnectionService for BlockingService {
        fn handle(&self, stream: UnixStream) {
            self.active.fetch_add(1, Ordering::AcqRel);
            while !self.release.load(Ordering::Acquire) {
                std::thread::yield_now();
            }
            let _ = stream.shutdown(Shutdown::Both);
            self.active.fetch_sub(1, Ordering::AcqRel);
        }
    }

    #[test]
    fn pool_admits_exactly_sixteen_active_connections() {
        let service = Arc::new(BlockingService {
            active: AtomicUsize::new(0),
            release: AtomicBool::new(false),
        });
        let mut pool = WorkerPool::start(&service).expect("bounded worker pool should start");
        let mut peers = Vec::new();
        for _ in 0..MAX_ACTIVE_HANDLERS {
            let (server, peer) = UnixStream::pair().expect("socket pair should be created");
            pool.dispatch(server)
                .expect("one of the sixteen slots should accept the stream");
            peers.push(peer);
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        while service.active.load(Ordering::Acquire) != MAX_ACTIVE_HANDLERS
            && Instant::now() < deadline
        {
            std::thread::yield_now();
        }
        assert_eq!(service.active.load(Ordering::Acquire), MAX_ACTIVE_HANDLERS);

        let (overflow, overflow_peer) =
            UnixStream::pair().expect("overflow socket pair should be created");
        assert!(
            pool.dispatch(overflow).is_err(),
            "the seventeenth stream must be rejected while all slots are busy"
        );

        service.release.store(true, Ordering::Release);
        pool.shutdown().expect("workers should stop cleanly");
        drop(peers);
        drop(overflow_peer);
    }
}
