use serde::{
	de::{DeserializeOwned, IntoDeserializer},
	Deserialize,
	Deserializer
};

/// Event model:
///
/// ```json
/// { "event": "idle" }
/// { "event": "property-change", id: number, name: "property-name", data: "property-value" }
/// ```
///
/// See https://mpv.io/manual/stable/#list-of-events.
#[derive(Debug, Deserialize)]
#[serde(tag = "event")]
pub enum MpvResponseEvent {
	#[serde(rename = "property-change")]
	PropertyChange {
		/// Id of the observer.
		id: i64,
		#[serde(deserialize_with = "MpvResponseEventPropertyName::deserialize_with_unknown")]
		name: MpvResponseEventPropertyName,
		#[serde(default)]
		data: serde_json::Value
	},
	#[serde(rename = "log-message")]
	LogMessage {}, // TOOD
	// media
	#[serde(rename = "start-file")]
	StartFile { playlist_entry_id: i64 },
	#[serde(rename = "end-file")]
	EndFile {}, // TODO
	#[serde(rename = "file-loaded")]
	FileLoaded,
	#[serde(rename = "seek")]
	Seek,
	#[serde(rename = "playback-restart")]
	PlaybackRestart,
	#[serde(rename = "shutdown")]
	Shutdown,
	#[serde(rename = "audio-reconfig")]
	AudioReconfig,
	#[serde(rename = "video-reconfig")]
	VideoReconfig,

	// deprecated
	// #[serde(rename = "tracks-changed")]
	// TracksChanged,
	// #[serde(rename = "track-switched")]
	// TrackSwitched,
	// #[serde(rename = "pause")]
	// Pause,
	// #[serde(rename = "unpause")]
	// Unpause,
	// #[serde(rename = "metadata-update")]
	// MetadataUpdate,
	// #[serde(rename = "idle")]
	// Idle,
	// #[serde(rename = "tick")]
	// Tick,
	// #[serde(rename = "chapter-change")]
	// ChapterChange

	// unknown
	#[serde(other)]
	Unknown
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]

pub enum MpvResponseEventPropertyName {
	Volume,
	Filename,
	#[serde(rename = "filename/no-ext")]
	FilenameNoExt,
	// unknown
	#[serde(skip_deserializing)]
	Unknown(String)
}
impl MpvResponseEventPropertyName {
	pub fn deserialize_with_unknown<'de, D: Deserializer<'de>>(
		deserializer: D
	) -> Result<Self, D::Error> {
		let string = String::deserialize(deserializer)?;

		match Self::deserialize(IntoDeserializer::<'de, D::Error>::into_deserializer(
			string.as_str()
		)) {
			Ok(value) => Ok(value),
			Err(_) => Ok(Self::Unknown(string))
		}
	}
}

/// Result model:
///
/// ```json
/// { "error": "success" | "invalid paramter" | "...", "data"?: "value" | 123 | true | null, "request_id"?: 123 }
/// ```
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum MpvResponseResult<Data: DeserializeOwned = serde_json::Value> {
	Success {
		error: MpvResponseResultSuccess,
		#[serde(bound = "")]
		data: Data,
		request_id: Option<i64>
	},
	Error {
		error: MpvResponseResultError,
		request_id: Option<i64>
	}
}
impl<D: DeserializeOwned> MpvResponseResult<D> {
	pub fn request_id(&self) -> Option<i64> {
		match self {
			MpvResponseResult::Success { request_id, .. } => *request_id,
			MpvResponseResult::Error { request_id, .. } => *request_id
		}
	}
}

#[derive(Debug, Deserialize)]
pub enum MpvResponseResultSuccess {
	#[serde(rename = "success")]
	Success
}

#[derive(Debug, Deserialize)]
pub enum MpvResponseResultError {
	#[serde(rename = "invalid parameter")]
	InvalidParameter,
	#[serde(rename = "property unavailable")]
	PropertyUnavailable,
	#[serde(rename = "property not found")]
	PropertyNotFound
}

/// Either a mpv event or a mpv result.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum MpvResponse<ResponseData: DeserializeOwned = serde_json::Value> {
	Event(MpvResponseEvent),
	#[serde(bound = "")]
	Result(MpvResponseResult<ResponseData>)
}

#[cfg(test)]
mod test {
	use super::{MpvResponseEvent, MpvResponseEventPropertyName};

	#[test]
	fn test_mpv_response_event_property_change() {
		let json = serde_json::json!(
			{
				"event": "property-change",
				"id": 1,
				"name": "filename"
			}
		);

		let response: MpvResponseEvent = serde_json::from_value(json).unwrap();

		match response {
			MpvResponseEvent::PropertyChange {
				id: 1,
				name: MpvResponseEventPropertyName::Filename,
				data: serde_json::Value::Null
			} => (),
			me => panic!("Expected MpvResponseEvent::PropertyChange {{ id: 1, name: Filename, data: null }} but found {:?}", me)
		}
	}

	#[test]
	fn test_mpv_response_event_property_change_name_unknown_parse() {
		let json = serde_json::json!(
			{
				"event": "property-change",
				"id": 1,
				"name": "whatever-unknown-thing"
			}
		);

		let response: MpvResponseEvent = serde_json::from_value(json).unwrap();

		match response {
			MpvResponseEvent::PropertyChange {
				id: 1,
				name: MpvResponseEventPropertyName::Unknown(ref name),
				data: serde_json::Value::Null
			} if name == "whatever-unknown-thing" => (),
			me => panic!("Expected MpvResponseEvent::PropertyChange {{ id: 1, name: Unknown(\"whatever-unknown-thing\"), data: null }} but found {:?}", me)
		}
	}

	#[test]
	fn test_mpv_response_event_unknown_parse() {
		let json = serde_json::json!(
			{
				"event": "idle"
			}
		);

		let response: MpvResponseEvent = serde_json::from_value(json).unwrap();

		// TODO: Is this feasible?
		// match response {
		// 	MpvResponseEvent::Unknown(ref tag) if tag == "idle" => (),
		// 	me => panic!("Expected MpvResponseEvent::Unknown(\"idle\") but found {:?}", me)
		// }

		match response {
			MpvResponseEvent::Unknown => (),
			me => panic!("Expected MpvResponseEvent::Unknown() but found {:?}", me)
		}
	}
}
