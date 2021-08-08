use std::borrow::Cow;

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

pub struct MpvGetVersion;
impl MpvCommand for MpvGetVersion {
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

pub struct MpvGetProperty<P: MpvProperty>(P);
impl<P: MpvProperty> MpvGetProperty<P> {
	pub fn new(property: P) -> Self {
		MpvGetProperty(property)
	}
}
impl<P: MpvProperty> MpvCommand for MpvGetProperty<P> {
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


pub struct MpvSetProperty<P: MpvProperty>(P, P::Value);
impl<P: MpvProperty> MpvSetProperty<P> {
	pub fn new(property: P, value: P::Value) -> Self {
		MpvSetProperty(property, value)
	}
}
impl<P: MpvProperty> MpvCommand for MpvSetProperty<P> {
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

pub struct MpvObserveProperty<P: MpvProperty>(u32, P);
impl<P: MpvProperty> MpvObserveProperty<P> {
	pub fn new(observer_id: u32, property: P) -> Self {
		MpvObserveProperty(observer_id, property)
	}
}
impl<P: MpvProperty> MpvCommand for MpvObserveProperty<P> {
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

pub struct MpvUnobserveProperty(u32);
impl MpvUnobserveProperty {
	pub fn new(observer_id: u32) -> Self {
		MpvUnobserveProperty(observer_id)
	}
}
impl MpvCommand for MpvUnobserveProperty {
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

pub struct MpvLoadfile<'a>(pub Cow<'a, str>);
impl<'a> MpvLoadfile<'a> {
	pub fn new(file_path: Cow<'a, str>) -> Self {
		MpvLoadfile(file_path)
	}
}
impl<'a> MpvCommand for MpvLoadfile<'a> {
	type Data = Option<()>;
	type Error = std::convert::Infallible;
	type ParsedData = Self::Data;

	fn write_args(&self, mut w: impl std::io::Write) -> std::io::Result<()> {
		write!(w, "\"loadfile\",{}", self.0)
	}

	fn parse_data(&self, data: Self::Data) -> Result<Self::ParsedData, Self::Error> {
		Ok(data)
	}
}

pub struct MpvStop(pub bool);
impl MpvStop {
	pub fn new(keep_playlist: bool) -> Self {
		MpvStop(keep_playlist)
	}
}
impl MpvCommand for MpvStop {
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
