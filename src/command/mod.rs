use std::{io, num::NonZeroI64};

use serde::de::DeserializeOwned;

pub mod commands;
pub mod property;
pub mod response;

/// Trait for mpv command definition.
///
/// Command model:
///
/// ```json
/// { "command": ["name", "arg1", "arg2"], "request_id"?: 123 }
/// ```
pub trait MpvCommand {
	/// The response `data` field type.
	///
	/// Usually `serde_json::Value`.
	type Data: DeserializeOwned;
	/// The output of `parse_data`.
	type ParsedData;
	/// The error produced while parsing `Self::Data`.
	type Error: std::error::Error;

	/// Formats command arguments into a formatter.
	///
	/// The arguments must be formatted as a valid JSON array with the enclosing paren (`[]`) symbols removed.
	///
	/// For example to send `{ "command": ["name", "arg1", "arg2"], "request_id"?: 123 }` this method should
	/// format into the formatter `"name", "arg1", "arg2"`.
	fn write_args(&self, w: impl io::Write) -> io::Result<()>;

	/// Parses data from a response "data" field. The data is guaranteed to be a valid JSON value.
	fn parse_data(&self, data: Self::Data) -> Result<Self::ParsedData, Self::Error>;
}

pub struct MpvCommandContext<'args, C: MpvCommand + ?Sized> {
	command: &'args C,
	request_id: Option<NonZeroI64>
}
impl<'a, C: MpvCommand + ?Sized> MpvCommandContext<'a, C> {
	pub fn new(command: &'a C, request_id: Option<NonZeroI64>) -> Self {
		MpvCommandContext {
			command,
			request_id
		}
	}

	pub fn write(&self, mut w: impl io::Write) -> io::Result<()> {
		let request_id = self.request_id.map(|n| n.get()).unwrap_or(0);

		write!(w, "{{\"request_id\":{},\"command\":[", request_id)?;
		self.command.write_args(&mut w)?;
		write!(w, "]}}",)?;

		Ok(())
	}

	pub fn writeln(&self, mut w: impl io::Write) -> io::Result<()> {
		self.write(&mut w)?;
		writeln!(w)?;

		Ok(())
	}
}
