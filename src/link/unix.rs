use std::{
	convert::TryFrom,
	fs,
	io::{self, Read, Write},
	path::Path,
	process::{Child, Command, Stdio}
};

use std::os::unix::{fs::FileTypeExt, net::UnixStream, prelude::AsRawFd};

use super::{MpvLinkDeinitError, MpvLinkInitError};

enum MpvLinkInner {
	/// Link has been closed.
	Closed,
	/// The mpv process is separate.
	Socket { socket: UnixStream },
	/// The mpv process is a child of this process.
	Child { child: Child, socket: UnixStream }
}
pub struct MpvLink {
	inner: MpvLinkInner
}
impl MpvLink {
	/// Spawns a new child process and uses the `input-ipc-client` option to pass it a socket.
	pub fn spawn_client() -> Result<Self, MpvLinkInitError> {
		let (socket, mpv_socket) = UnixStream::pair().map_err(MpvLinkInitError::SocketPair)?;

		// unset cloexec so the child inherits the socket
		unsafe {
			let res = libc::ioctl(mpv_socket.as_raw_fd(), libc::FIONCLEX);
			if res < 0 {
				return Err(MpvLinkInitError::Cloexec(io::Error::last_os_error()))
			}
		}

		let child = {
			let socket_arg = format!("--input-ipc-client=fd://{}", mpv_socket.as_raw_fd());

			Command::new("mpv")
				.arg("--idle")
				.arg("--no-terminal")
				.arg(&socket_arg)
				.stdin(Stdio::null())
				.stdout(Stdio::null())
				.stderr(Stdio::null())
				.spawn()
				.map_err(MpvLinkInitError::Spawn)?
		};
		std::mem::drop(mpv_socket);

		log::info!("Spawned mpv with pid: {}", child.id());

		let me = MpvLink {
			inner: MpvLinkInner::Child { child, socket }
		};

		Ok(me)
	}

	/// Spawns a new child process and uses the `input-ipc-server` option to pass it a path where to create a socket.
	pub fn spawn_server(path: &Path) -> Result<Self, MpvLinkInitError> {
		if fs::metadata(path)
			.map(|m| m.file_type().is_socket())
			.unwrap_or(false)
		{
			log::info!("Removing existing socket at {}", path.display());
			fs::remove_file(path).map_err(MpvLinkInitError::RemovePrevious)?;
		}

		let child = {
			let socket_arg = format!("--input-ipc-server={}", path.display());

			Command::new("mpv")
				.arg("--idle")
				.arg("--no-terminal")
				.arg(&socket_arg)
				.stdin(Stdio::null())
				.stdout(Stdio::null())
				.stderr(Stdio::null())
				.spawn()
				.map_err(MpvLinkInitError::Spawn)?
		};

		log::info!("Spawned mpv with pid: {}", child.id());

		let socket = loop {
			match UnixStream::connect(path) {
				Ok(socket) => break socket,
				Err(err) if err.kind() == io::ErrorKind::NotFound => {
					std::thread::yield_now();
				}
				Err(err) => return Err(MpvLinkInitError::Connect(err))
			}
		};

		let me = MpvLink {
			inner: MpvLinkInner::Child { child, socket }
		};

		Ok(me)
	}

	/// Connects to an existing process spawned with `input-ipc-server` option by opening the socket.
	pub fn connect(path: &Path) -> Result<Self, MpvLinkInitError> {
		let socket = UnixStream::connect(path).map_err(MpvLinkInitError::Connect)?;

		let me = MpvLink {
			inner: MpvLinkInner::Socket { socket }
		};

		Ok(me)
	}

	pub fn set_nonblocking(&mut self, nonblocking: bool) -> Result<(), io::Error> {
		match &mut self.inner {
			MpvLinkInner::Closed => panic!("Mpv link closed"),
			MpvLinkInner::Child { socket, .. } => {
				socket.set_nonblocking(nonblocking)?;
			}
			MpvLinkInner::Socket { socket } => {
				socket.set_nonblocking(nonblocking)?;
			}
		}

		Ok(())
	}

	/// Blocks until reading becomes available on `self.stream()`.
	///
	/// If timeout is not `None` then returns `Err(TimedOut)` on timeout.
	pub fn wait_read(&self, timeout: Option<std::time::Duration>) -> Result<(), io::Error> {
		let fd = match &self.inner {
			MpvLinkInner::Closed => panic!("Mpv link closed"),
			MpvLinkInner::Child { socket, .. } => socket.as_raw_fd(),
			MpvLinkInner::Socket { socket } => socket.as_raw_fd()
		};

		let mut info = libc::pollfd {
			fd,
			events: libc::POLLIN,
			revents: 0
		};

		let timeout = match timeout {
			None => -1,
			Some(timeout) => libc::c_int::try_from(timeout.as_secs()).unwrap_or(libc::c_int::MAX)
		};

		let result = unsafe { libc::poll(&mut info, 1, timeout) };

		if result < 0 {
			return Err(io::Error::last_os_error())
		} else if result == 0 {
			return Err(io::ErrorKind::TimedOut.into())
		}

		if info.revents & libc::POLLNVAL != 0 {
			return Err(io::ErrorKind::InvalidInput.into())
		} else if info.revents & libc::POLLERR != 0 {
			// No idea
			return Err(io::ErrorKind::Other.into())
		} else if info.revents & libc::POLLHUP != 0 {
			// pass
		}

		Ok(())
	}

	/// Returns the RW stream for this link.
	///
	/// ### Panic
	/// Panics is `self` has been deinitialized.
	pub fn stream(&mut self) -> impl Read + Write + '_ {
		match &mut self.inner {
			MpvLinkInner::Closed => panic!("Mpv link closed"),
			MpvLinkInner::Child { socket, .. } => socket,
			MpvLinkInner::Socket { socket } => socket
		}
	}

	/// Returns `true` if `self.deinit()` has been called.
	pub fn is_deinit(&self) -> bool {
		match self.inner {
			MpvLinkInner::Closed => true,
			_ => false
		}
	}

	/// Deinitializes `self`.
	///
	/// If `self` has been deinitialized returns `Ok(())`.
	pub fn deinit(&mut self) -> Result<(), MpvLinkDeinitError> {
		let inner = std::mem::replace(&mut self.inner, MpvLinkInner::Closed);

		fn deinit_socket(socket: UnixStream) -> Result<(), MpvLinkDeinitError> {
			log::info!("Shutting down and closing socket");
			let _ = socket
				.shutdown(std::net::Shutdown::Both)
				.map_err(MpvLinkDeinitError::Shutdown)?;
			std::mem::drop(socket);

			Ok(())
		}

		match inner {
			MpvLinkInner::Closed => Ok(()),
			MpvLinkInner::Socket { socket } => deinit_socket(socket),
			MpvLinkInner::Child {
				mut socket,
				mut child
			} => {
				// write quit command to make sure mpv quits
				let quit_result = socket.write(b"quit\n");
				log::info!("Wrote quit command: {:?}", quit_result);

				let _ = deinit_socket(socket);

				log::info!("Waiting for mpv child to exit");
				child.wait().map_err(MpvLinkDeinitError::Wait)?;
				std::mem::drop(child);

				Ok(())
			}
		}
	}
}
impl Drop for MpvLink {
	fn drop(&mut self) {
		self.deinit().expect("Failed to deinit MpvLink in drop")
	}
}
