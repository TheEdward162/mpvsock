use std::borrow::Cow;

use crate::model::FileloadInfo;

use super::property::MpvProperty;

use super::MpvCommand;

impl MpvCommand for str {
	type Data = Option<serde_json::Value>;
	type Error = std::convert::Infallible;
	type ParsedData = serde_json::Value;

	fn write_args(&self, mut w: impl std::io::Write) -> std::io::Result<()> {
		write!(w, "{}", self)
	}

	fn parse_data(&self, data: Self::Data) -> Result<Self::ParsedData, Self::Error> {
		let value = match data {
			None => serde_json::Value::Null,
			Some(value) => value
		};

		Ok(value)
	}
}

pub struct CmdGetVersion;
impl MpvCommand for CmdGetVersion {
	type Data = u32;
	type Error = serde_json::Error;
	type ParsedData = (u16, u16);

	fn write_args(&self, mut w: impl std::io::Write) -> std::io::Result<()> {
		write!(w, "\"get_version\"")
	}

	fn parse_data(&self, data: Self::Data) -> Result<Self::ParsedData, Self::Error> {
		let major = (data >> 16) & 0xFFFF;
		let minor = data & 0xFFFF;

		Ok((major as u16, minor as u16))
	}
}

pub struct CmdGetProperty<P: MpvProperty>(P);
impl<P: MpvProperty> CmdGetProperty<P> {
	pub fn new(property: P) -> Self {
		CmdGetProperty(property)
	}
}
impl<P: MpvProperty> MpvCommand for CmdGetProperty<P> {
	type Data = P::Value;
	type Error = std::convert::Infallible;
	type ParsedData = Self::Data;

	fn write_args(&self, mut w: impl std::io::Write) -> std::io::Result<()> {
		write!(w, "\"get_property\",\"{}\"", self.0.name())
	}

	fn parse_data(&self, data: Self::Data) -> Result<Self::ParsedData, Self::Error> {
		Ok(data)
	}
}


pub struct CmdSetProperty<P: MpvProperty>(P, P::Value);
impl<P: MpvProperty> CmdSetProperty<P> {
	pub fn new(property: P, value: P::Value) -> Self {
		CmdSetProperty(property, value)
	}
}
impl<P: MpvProperty> MpvCommand for CmdSetProperty<P> {
	type Data = Option<()>;
	type Error = serde_json::Error;
	type ParsedData = Self::Data;

	fn write_args(&self, mut w: impl std::io::Write) -> std::io::Result<()> {
		write!(w, "\"set_property\",\"{}\",", self.0.name())?;
		serde_json::to_writer(w, &self.1)?;

		Ok(())
	}

	fn parse_data(&self, data: Self::Data) -> Result<Self::ParsedData, Self::Error> {
		Ok(data)
	}
}

pub struct CmdObserveProperty<P: MpvProperty>(u32, P);
impl<P: MpvProperty> CmdObserveProperty<P> {
	pub fn new(observer_id: u32, property: P) -> Self {
		CmdObserveProperty(observer_id, property)
	}
}
impl<P: MpvProperty> MpvCommand for CmdObserveProperty<P> {
	type Data = Option<()>;
	type Error = std::convert::Infallible;
	type ParsedData = Self::Data;

	fn write_args(&self, mut w: impl std::io::Write) -> std::io::Result<()> {
		write!(w, "\"observe_property\",{},\"{}\"", self.0, self.1.name())
	}

	fn parse_data(&self, data: Self::Data) -> Result<Self::ParsedData, Self::Error> {
		Ok(data)
	}
}

pub struct CmdUnobserveProperty(u32);
impl CmdUnobserveProperty {
	pub fn new(observer_id: u32) -> Self {
		CmdUnobserveProperty(observer_id)
	}
}
impl MpvCommand for CmdUnobserveProperty {
	type Data = Option<()>;
	type Error = std::convert::Infallible;
	type ParsedData = Self::Data;

	fn write_args(&self, mut w: impl std::io::Write) -> std::io::Result<()> {
		write!(w, "\"unobserve_property\",{}", self.0)
	}

	fn parse_data(&self, data: Self::Data) -> Result<Self::ParsedData, Self::Error> {
		Ok(data)
	}
}

pub struct CmdLoadfile<'a>(pub Cow<'a, str>);
impl<'a> CmdLoadfile<'a> {
	pub fn new(file_path: Cow<'a, str>) -> Self {
		CmdLoadfile(file_path)
	}
}
impl<'a> MpvCommand for CmdLoadfile<'a> {
	type Data = FileloadInfo;
	type Error = std::convert::Infallible;
	type ParsedData = Self::Data;

	fn write_args(&self, mut w: impl std::io::Write) -> std::io::Result<()> {
		write!(w, "\"loadfile\",\"{}\"", self.0)
	}

	fn parse_data(&self, data: Self::Data) -> Result<Self::ParsedData, Self::Error> {
		Ok(data)
	}
}

pub struct CmdStop(pub bool);
impl CmdStop {
	pub fn new(keep_playlist: bool) -> Self {
		CmdStop(keep_playlist)
	}
}
impl MpvCommand for CmdStop {
	type Data = Option<()>;
	type Error = std::convert::Infallible;
	type ParsedData = Self::Data;

	fn write_args(&self, mut w: impl std::io::Write) -> std::io::Result<()> {
		if self.0 {
			write!(w, "\"stop\",\"keep-playlist\"")
		} else {
			write!(w, "\"stop\"")
		}
	}

	fn parse_data(&self, data: Self::Data) -> Result<Self::ParsedData, Self::Error> {
		Ok(data)
	}
}
