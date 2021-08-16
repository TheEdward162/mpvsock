use std::{io::{self, Write}, num::NonZeroI64, path::Path};

use serde::de::DeserializeOwned;
use thiserror::Error;

use crate::{command::MpvCommandRaw, response_buffer::ResponseBuffer};

#[cfg(unix)]
pub mod unix;

#[cfg(unix)]
type InnerLink = unix::MpvLink;

use crate::command::{
	response::{MpvResponse, MpvResponseEvent, MpvResponseResult, MpvResponseResultError},
	MpvCommand
};

#[derive(Debug, Error)]
pub enum CommandError<E: std::error::Error> {
	#[error(transparent)]
	SendError(#[from] SendError),
	#[error(transparent)]
	ReceiveError(#[from] ReceiveError),
	#[error("Received error response: {0:?}")]
	ResultError(MpvResponseResultError),
	#[error("Error while parsing response data: {0}")]
	DataParseError(E)
}

#[derive(Debug, Error)]
pub enum SendError {
	#[error("Could not write into the stream: {0}")]
	Io(#[from] std::io::Error)
}

#[derive(Debug, Error)]
pub enum ReceiveError {
	#[error("Could not read from the stream: {0}")]
	Io(#[from] std::io::Error),
	#[error("Could not deserialize response: {0}")]
	Deserialize(#[from] serde_json::Error),
	#[error("Expected request_id = {expected} but found request_id = {found}")]
	RequestIdMismatch { expected: i64, found: i64 },
	#[error("Expected only events but found a result response")]
	UnexpectedResponseResult(MpvResponseResult)
}

#[derive(Debug, Error)]
pub enum MpvLinkInitError {
	#[cfg(unix)]
	#[error("Failed to create socket pair")]
	SocketPair(io::Error),
	#[cfg(unix)]
	#[error("Failed to clear CLOEXEC flag")]
	Cloexec(io::Error),

	#[error("Failed to set channel to nonblocking")]
	Nonblocking(io::Error),
	#[error("Failed to spawn process")]
	Spawn(io::Error),
	#[error("Failed to connect to server socket")]
	Connect(io::Error),
	#[error("Failed to remove previous socket")]
	RemovePrevious(io::Error)
}

#[derive(Debug, Error)]
pub enum MpvLinkDeinitError {
	#[error("Failed to shutdown socket")]
	Shutdown(io::Error),
	#[error("Failed to wait for the child process")]
	Wait(io::Error)
}

pub struct MpvLink {
	inner: InnerLink,
	current_id: NonZeroI64,
	response_buffer: ResponseBuffer,
	event_queue: Vec<MpvResponseEvent>
}
impl MpvLink {
	const NONZERO_ONE: NonZeroI64 = unsafe { NonZeroI64::new_unchecked(1) };

	fn new(mut inner: InnerLink) -> Result<Self, MpvLinkInitError> {
		inner
			.set_nonblocking(true)
			.map_err(MpvLinkInitError::Nonblocking)?;

		let me = MpvLink {
			inner,
			current_id: Self::NONZERO_ONE,
			response_buffer: ResponseBuffer::new(),
			event_queue: Vec::new()
		};

		Ok(me)
	}

	pub fn connect(socket_path: &Path) -> Result<Self, MpvLinkInitError> {
		let inner = InnerLink::connect(socket_path)?;

		Self::new(inner)
	}

	pub fn spawn_server(socket_path: &Path) -> Result<Self, MpvLinkInitError> {
		let inner = InnerLink::spawn_server(socket_path)?;

		Self::new(inner)
	}

	#[cfg(unix)]
	pub fn spawn_client() -> Result<Self, MpvLinkInitError> {
		let inner = InnerLink::spawn_client()?;

		Self::new(inner)
	}

	pub fn run_command<C: MpvCommand + ?Sized>(
		&mut self,
		command: &C
	) -> Result<C::ParsedData, CommandError<C::Error>> {
		let current_id = self.run_command_raw(command)?;

		let result = loop {
			let result = self.next_result::<C::Data>()?;
			match result.request_id() {
				Some(request_id) if request_id == current_id.get() => break result,
				request_id => {
					return Err(ReceiveError::RequestIdMismatch {
						expected: current_id.get(),
						found: request_id.unwrap_or(0)
					}
					.into())
				}
			}
		};

		match result {
			MpvResponseResult::Error { error, .. } => Err(CommandError::ResultError(error)),
			MpvResponseResult::Success { data, .. } => {
				let data = command
					.parse_data(data)
					.map_err(CommandError::DataParseError)?;

				Ok(data)
			}
		}
	}

	/// Runs a `MpvCommandRaw` and does not wait for the result.
	pub fn run_command_raw<C: MpvCommandRaw + ?Sized, E: std::error::Error>(
		&mut self,
		command: &C
	) -> Result<NonZeroI64, CommandError<E>> {
		let current_id = {
			let current = self.current_id;
			self.current_id =
				NonZeroI64::new(self.current_id.get().wrapping_add(1)).unwrap_or(Self::NONZERO_ONE);
			current
		};

		self.send_command(command, current_id)?;

		Ok(current_id)
	}

	/// Polls for events which are added to the internal queue.
	///
	/// Returns the currently queued events.
	pub fn poll_events(&mut self) -> Result<&[MpvResponseEvent], ReceiveError> {
		loop {
			match self.next_response()? {
				None => break,
				Some(response) => match response {
					MpvResponse::Event(event) => {
						log::trace!("Queued event: {:?}", event);
						self.event_queue.push(event);
					}
					MpvResponse::Result(result) => {
						return Err(ReceiveError::UnexpectedResponseResult(result))
					}
				}
			};
		}
		self.response_buffer.shift();

		Ok(&self.event_queue)
	}

	pub fn drain_events(&mut self) -> impl Iterator<Item = MpvResponseEvent> + '_ {
		self.event_queue.drain(..)
	}

	fn send_command<C: MpvCommandRaw + ?Sized>(
		&mut self,
		command: &C,
		current_id: NonZeroI64
	) -> Result<(), SendError> {
		if log::log_enabled!(log::Level::Debug) {
			let mut buffer = Vec::new();
			command.write(&mut buffer, Some(current_id))?;

			match std::str::from_utf8(&buffer) {
				Ok(command) => log::debug!("Sending command: {}", command),
				Err(_) => log::debug!("Sending command: {:?}", buffer)
			};
		}

		let mut stream = self.inner.stream();
		command.write(&mut stream, Some(current_id))?;
		writeln!(stream)?;

		Ok(())
	}

	fn next_response<ResponseData: DeserializeOwned>(
		&mut self
	) -> Result<Option<MpvResponse<ResponseData>>, ReceiveError> {
		log::trace!("Waiting for next response");
		let line = match self.response_buffer.consume_line() {
			Some(line) => line,
			None => {
				self.response_buffer.read_nonblocking(self.inner.stream())?;
				match self.response_buffer.consume_line() {
					Some(line) => line,
					None => return Ok(None)
				}
			}
		};

		let response: MpvResponse<ResponseData> = serde_json::from_slice(line)?;

		Ok(Some(response))
	}

	fn next_result<Data: DeserializeOwned>(
		&mut self
	) -> Result<MpvResponseResult<Data>, ReceiveError> {
		log::trace!("Waiting for next result");
		let result = loop {
			match self.next_response()? {
				// TODO: Handle deadlock from issuing a non-result command through non-raw interface throuw timeout?
				None => self.inner.wait_read(None)?,
				Some(response) => match response {
					MpvResponse::Event(event) => {
						log::trace!("Queued event: {:?}", event);
						self.event_queue.push(event);
					}
					MpvResponse::Result(result) => break result
				}
			};
		};
		self.response_buffer.shift();

		Ok(result)
	}
}
