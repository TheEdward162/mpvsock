use std::num::NonZeroI64;

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
	type Data;
	/// The error produced while parsing `Self::Data`.
	type Error: std::error::Error;

	/// Formats command arguments into a formatter.
	fn fmt_args(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result;

	/// Parses data from a response "data" field. The data is guaranteed to be a valid JSON value.
	fn parse_data(&self, value: serde_json::Value) -> Result<Self::Data, Self::Error>;
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
}
impl<'a, C: MpvCommand + ?Sized> std::fmt::Display for MpvCommandContext<'a, C> {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		let request_id = self.request_id.map(|n| n.get()).unwrap_or(0);

		write!(f, "{{\"request_id\":{},\"command\":[", request_id)?;
		self.command.fmt_args(f)?;
		write!(f, "]}}",)?;

		Ok(())
	}
}

impl MpvCommand for str {
	type Data = serde_json::Value;
	type Error = std::convert::Infallible;

	fn fmt_args(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}", self)
	}

	fn parse_data(&self, value: serde_json::Value) -> Result<Self::Data, Self::Error> {
		Ok(value)
	}
}
