use std::{io, num::NonZeroI64};

use serde::de::DeserializeOwned;

pub mod commands;
pub mod property;
pub mod response;

/// Trait for mpv command definiton.
///
/// This encompasses both text and JSON commands.
pub trait MpvCommandRaw {
	/// Formats command into a stream.
	fn write(&self, w: impl io::Write, request_id: Option<NonZeroI64>) -> io::Result<()>;
}

/// Trait for mpv JSON command definition.
///
/// Command model:
///
/// ```json
/// { "command": ["name", "arg1", "arg2"], "request_id"?: 123 }
/// ```
pub trait MpvCommand: MpvCommandRaw {
	/// The response `data` field type.
	///
	/// Usually `serde_json::Value`.
	type Data: DeserializeOwned;
	/// The output of `parse_data`.
	type ParsedData;
	/// The error produced while parsing `Self::Data`.
	type Error: std::error::Error;

	/// Formats command arguments into a stream.
	///
	/// The arguments must be formatted as a valid JSON array with the enclosing paren (`[]`) symbols removed.
	///
	/// For example to send `{ "command": ["name", "arg1", "arg2"], "request_id"?: 123 }` this method should
	/// write into the stream `"name", "arg1", "arg2"`.
	///
	/// This method is only called from the default implementation of `write_raw`.
	fn write_args(&self, w: impl io::Write) -> io::Result<()>;

	/// Parses data from a response "data" field. The data is guaranteed to be a valid JSON value.
	fn parse_data(&self, data: Self::Data) -> Result<Self::ParsedData, Self::Error>;
}

impl<T: MpvCommand> MpvCommandRaw for T {
	fn write(&self, mut w: impl io::Write, request_id: Option<NonZeroI64>) -> io::Result<()>  {
		write!(w, "{{\"request_id\":{},\"command\":[", request_id.map(|n| n.get()).unwrap_or(0))?;
		self.write_args(&mut w)?;
		write!(w, "]}}",)?;

		Ok(())
	}
}