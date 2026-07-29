use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::fs::{FileTypeExt, OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use fs2::FileExt;
use signal_hook::consts::{SIGINT, SIGTERM};
use socket2::{Domain, SockAddr, Socket, Type};

use crate::broker::Broker;
use crate::error::{Error, Result};
use crate::model::MAX_LINE_BYTES;
use crate::niri::NiriClient;
use crate::paths::Paths;
use crate::shell::ShellController;

use super::dispatch::Dispatcher;
use super::protocol::process_line;
use super::workers::{ConnectionService, LockedService, WorkerPool};

const READ_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_PENDING_CONNECTIONS: i32 = 16;
const ACCEPT_POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Serves the Weyriva control socket until SIGINT or SIGTERM.
///
/// # Errors
///
/// Returns an error for unsafe paths, socket setup, signal handling, or broker failures.
pub fn serve(paths: &Paths, broker: Broker) -> Result<()> {
    let shutdown = Arc::new(AtomicBool::new(false));
    for signal in [SIGINT, SIGTERM] {
        signal_hook::flag::register(signal, Arc::clone(&shutdown))
            .map_err(|error| Error::io("cannot register daemon signal handler", &error))?;
    }
    serve_until(paths, broker, &shutdown)
}

/// Serves the Weyriva control socket until the supplied flag is set.
///
/// # Errors
///
/// Returns an error for unsafe paths, socket setup, client I/O, or broker failures.
pub fn serve_until(paths: &Paths, mut broker: Broker, shutdown: &AtomicBool) -> Result<()> {
    secure_runtime_dir(&paths.runtime_dir)?;
    let daemon_lock = acquire_daemon_lock(&paths.daemon_lock_file())?;
    let migration_lock = crate::legacy_migration::MigrationLock::acquire_after_daemon(paths)?;
    crate::legacy_migration::migrate(paths)?;
    drop(migration_lock);
    let socket_path = paths.socket_file();
    remove_stale_socket(&socket_path)?;
    let listener = bind_listener(&socket_path)?;
    let socket_guard = SocketGuard::new(socket_path);
    broker.start_enabled()?;
    let service = Arc::new(DaemonService {
        broker: LockedService::new(broker),
        shell: ShellController::from_environment(),
        niri: NiriClient::system(),
    });
    let mut workers = WorkerPool::start(&service)?;
    let accept = accept_until_shutdown(&listener, &mut workers, shutdown);
    let worker_cleanup = workers.shutdown();
    let broker_cleanup = service
        .broker
        .lock()
        .and_then(|mut broker| broker.shutdown());
    drop(socket_guard);
    drop(daemon_lock);
    match (accept, worker_cleanup, broker_cleanup) {
        (Err(error), _, _) | (Ok(()), Err(error), _) | (Ok(()), Ok(()), Err(error)) => Err(error),
        (Ok(()), Ok(()), Ok(_)) => Ok(()),
    }
}

fn accept_until_shutdown(
    listener: &UnixListener,
    workers: &mut WorkerPool,
    shutdown: &AtomicBool,
) -> Result<()> {
    while !shutdown.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, _address)) => {
                if let Err(stream) = workers.dispatch(stream) {
                    reject_busy(stream);
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(ACCEPT_POLL_INTERVAL);
            }
            Err(error) => return Err(Error::io("IPC socket accept failed", &error)),
        }
    }
    Ok(())
}

struct DaemonService {
    broker: LockedService<Broker>,
    shell: ShellController,
    niri: NiriClient,
}

impl ConnectionService for DaemonService {
    fn handle(&self, stream: UnixStream) {
        let _ = self.handle_connection(stream);
    }
}

impl DaemonService {
    fn handle_connection(&self, mut stream: UnixStream) -> Result<()> {
        let line = read_request(&stream)?;
        if line.is_empty() {
            return Ok(());
        }
        let mut broker = self.broker.lock()?;
        let mut dispatcher = Dispatcher::new(&mut broker, &self.shell, &self.niri);
        stream
            .write_all(&process_line(&line, &mut dispatcher))
            .map_err(|error| Error::io("cannot write IPC response", &error))
    }
}

fn read_request(stream: &UnixStream) -> Result<Vec<u8>> {
    stream
        .set_read_timeout(Some(READ_TIMEOUT))
        .and_then(|()| stream.set_write_timeout(Some(READ_TIMEOUT)))
        .map_err(|error| Error::io("cannot set IPC client timeout", &error))?;
    let reader_stream = stream
        .try_clone()
        .map_err(|error| Error::io("cannot clone IPC socket", &error))?;
    let mut line = Vec::new();
    BufReader::new(reader_stream)
        .take(u64::try_from(MAX_LINE_BYTES).unwrap_or(u64::MAX) + 1)
        .read_until(b'\n', &mut line)
        .map_err(|error| Error::io("cannot read IPC socket", &error))?;
    Ok(line)
}

fn reject_busy(mut stream: UnixStream) {
    let _ = stream.set_write_timeout(Some(Duration::from_millis(100)));
    let _ = stream.write_all(
        b"{\"id\":null,\"error\":{\"code\":\"server_busy\",\"message\":\"all 16 IPC handlers are busy\"}}\n",
    );
}

fn secure_runtime_dir(path: &Path) -> Result<()> {
    fs::create_dir_all(path)
        .map_err(|error| Error::io("cannot create Weyriva runtime directory", &error))?;
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| Error::io("cannot inspect Weyriva runtime directory", &error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::new(
            "unsafe_runtime",
            "Weyriva runtime path must be a regular directory",
        ));
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
        .map_err(|error| Error::io("cannot secure Weyriva runtime directory", &error))
}

fn bind_listener(path: &Path) -> Result<UnixListener> {
    let socket = Socket::new(Domain::UNIX, Type::STREAM, None)
        .map_err(|error| Error::io("cannot create IPC socket", &error))?;
    socket
        .bind(
            &SockAddr::unix(path)
                .map_err(|error| Error::io("cannot address IPC socket", &error))?,
        )
        .map_err(|error| Error::io("cannot bind IPC socket", &error))?;
    socket
        .listen(MAX_PENDING_CONNECTIONS)
        .and_then(|()| socket.set_nonblocking(true))
        .map_err(|error| Error::io("cannot listen on IPC socket", &error))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|error| Error::io("cannot secure IPC socket", &error))?;
    let descriptor: OwnedFd = socket.into();
    Ok(descriptor.into())
}

fn remove_stale_socket(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_socket() => fs::remove_file(path)
            .map_err(|error| Error::io("cannot remove stale IPC socket", &error)),
        Ok(_) => Err(Error::new(
            "unsafe_socket",
            format!("refusing to replace non-socket path: {}", path.display()),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Error::io("cannot inspect IPC socket", &error)),
    }
}

fn acquire_daemon_lock(path: &Path) -> Result<File> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            return Err(Error::new(
                "unsafe_lock",
                format!("daemon lock is not a regular file: {}", path.display()),
            ));
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(Error::io("cannot inspect daemon lock", &error)),
    }
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .open(path)
        .map_err(|error| Error::io("cannot open daemon lock", &error))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|error| Error::io("cannot secure daemon lock", &error))?;
    file.try_lock_exclusive().map_err(|error| {
        if error.kind() == std::io::ErrorKind::WouldBlock {
            Error::new("daemon_running", "another Weyriva daemon is running")
        } else {
            Error::io("cannot lock daemon state", &error)
        }
    })?;
    Ok(file)
}

struct SocketGuard {
    path: PathBuf,
}

impl SocketGuard {
    const fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

impl Drop for SocketGuard {
    fn drop(&mut self) {
        if fs::symlink_metadata(&self.path).is_ok_and(|metadata| metadata.file_type().is_socket()) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use crate::legacy_migration::MigrationLock;
    use crate::paths::Paths;

    use super::{acquire_daemon_lock, secure_runtime_dir};

    #[test]
    fn daemon_lock_remains_held_across_migration_lock_handoff() {
        let temporary = tempdir().expect("temporary directory should be created");
        let paths = Paths::new(
            &temporary.path().join("config"),
            &temporary.path().join("state"),
            &temporary.path().join("data"),
            &temporary.path().join("runtime"),
        );
        secure_runtime_dir(&paths.runtime_dir).expect("runtime directory should be secured");
        let daemon =
            acquire_daemon_lock(&paths.daemon_lock_file()).expect("daemon lock should be acquired");
        let migration = MigrationLock::acquire_after_daemon(&paths)
            .expect("migration lock should follow daemon lock");

        let competing = acquire_daemon_lock(&paths.daemon_lock_file())
            .expect_err("a competing daemon must not enter during migration");
        assert_eq!(competing.code(), "daemon_running");
        drop(migration);
        let after_migration = acquire_daemon_lock(&paths.daemon_lock_file())
            .expect_err("daemon lock must remain held after migration completes");
        assert_eq!(after_migration.code(), "daemon_running");

        drop(daemon);
        acquire_daemon_lock(&paths.daemon_lock_file())
            .expect("daemon lock should be released only when the server exits");
    }
}
