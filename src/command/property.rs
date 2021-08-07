use std::borrow::Cow;

use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub trait MpvProperty {
	type Value: Serialize + DeserializeOwned;

	fn name(&self) -> Cow<'_, str>;
}

impl<'a> MpvProperty for &'a str {
	type Value = serde_json::Value;

	fn name(&self) -> Cow<'a, str> {
		Cow::Borrowed(self)
	}
}
impl<'a> MpvProperty for Cow<'a, str> {
	type Value = serde_json::Value;

	fn name(&self) -> Cow<'_, str> {
		match self {
			Cow::Borrowed(s) => Cow::Borrowed(s),
			Cow::Owned(ref s) => s.into()
		}
	}
}

macro_rules! impl_known_property {
	(
		pub enum $known_enum_name: ident;

		$(
			$name: ident: $property_name: literal, $value_type: ty
		),+ $(,)?
	) => {
		$(
			pub struct $name;
			impl MpvProperty for $name {
				type Value = $value_type;

				fn name(&self) -> Cow<'_, str> {
					Cow::Borrowed($property_name)
				}
			}
		)+

		#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
		pub enum $known_enum_name {
			$(
				#[serde(rename = $property_name)]
				$name
			),+
		}
		impl $known_enum_name {
			pub const fn proeprty_name(&self) -> &'static str {
				match self {
					$(
						$known_enum_name::$name => $property_name
					),+
				}
			}
		}
	};
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TrackId {
	Index(u32),
	Str(TrackIdStr)
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TrackIdStr {
	#[serde(rename = "auto")]
	Auto,
	#[serde(other)]
	Unknown
}

impl_known_property! {
	pub enum KnownMpvProperty;

	// f64
	Volume: "volume", f64,
	PercentPos: "percent-pos", f64,
	TimePos: "time-pos", f64,
	// String
	Path: "path", String,
	WorkingDirectory: "working-directory", String,
	MediaTitle: "media-title", String,
	Filename: "filename", String,
	// Track id
	Aid: "aid", TrackId,
	Vid: "vid", TrackId,
	Sid: "sid", TrackId,
	// bool
	Fullscreen: "fullscreen", bool,
	Pause: "pause", bool,
}
