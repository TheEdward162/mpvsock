use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct FileloadInfo {
	pub playlist_entry_id: i64
}


#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
enum TrackIdRepr {
	Index(u32),
	Bool(bool),
	Str(TrackIdReprStr)
}
#[derive(Debug, Serialize, Deserialize)]
enum TrackIdReprStr {
	#[serde(rename = "auto")]
	Auto,
	#[serde(rename = "no")]
	No
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(untagged)]
#[serde(from = "TrackIdRepr")]
#[serde(into = "TrackIdRepr")]
pub enum TrackId {
	Index(u32),
	Auto,
	None
}
impl Default for TrackId {
	fn default() -> Self {
		TrackId::Auto
	}
}
impl From<TrackId> for TrackIdRepr {
	fn from(id: TrackId) -> Self {
		match id {
			TrackId::Index(index) => TrackIdRepr::Index(index),
			TrackId::Auto => TrackIdRepr::Str(TrackIdReprStr::Auto),
			TrackId::None => TrackIdRepr::Bool(false)
		}
	}
}
impl From<TrackIdRepr> for TrackId {
	fn from(repr: TrackIdRepr) -> Self {
		match repr {
			TrackIdRepr::Index(index) => TrackId::Index(index),
			TrackIdRepr::Bool(false) => TrackId::None,
			_ => TrackId::Auto
		}
	}
}

#[cfg(test)]
mod test {
	//! Test `TrackId`s as seen in the wild.
	use serde_json::json;

	use super::TrackId;

	#[test]
	fn parse_track_id_index() {
		let value = json!(1);
		let track = serde_json::from_value::<TrackId>(value).unwrap();

		assert!(matches!(track, TrackId::Index(1)));
	}

	#[test]
	fn parse_track_id_auto() {
		let value = json!("auto");
		let track = serde_json::from_value::<TrackId>(value).unwrap();

		assert!(matches!(track, TrackId::Auto));
	}

	#[test]
	fn parse_track_id_false() {
		let value = json!(false);
		let track = serde_json::from_value::<TrackId>(value).unwrap();

		assert!(matches!(track, TrackId::None));
	}

	#[test]
	fn parse_track_id_no() {
		let value = json!("no");
		let track = serde_json::from_value::<TrackId>(value).unwrap();

		assert!(matches!(track, TrackId::Auto));
	}
}
