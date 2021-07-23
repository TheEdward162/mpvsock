use serde::Deserialize;

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
		name: MpvResponseEventPropertyName,
		data: serde_json::Value
	},
	#[serde(rename = "log-message")]
	LogMessage {}, // TOOD
	// media
	#[serde(rename = "start-file")]
	StartFile {
		playlist_entry_id: i64
	},
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
	#[serde(rename = "tracks-changed")]
	TracksChanged,
	#[serde(rename = "track-switched")]
	TrackSwitched,
	#[serde(rename = "pause")]
	Pause,
	#[serde(rename = "unpause")]
	Unpause,
	#[serde(rename = "metadata-update")]
	MetadataUpdate,
	#[serde(rename = "idle")]
	Idle,
	#[serde(rename = "tick")]
	Tick,
	#[serde(rename = "chapter-change")]
	ChapterChange
}

#[derive(Debug, Deserialize)]
pub enum MpvResponseEventPropertyName {
	#[serde(rename = "volume")]
	Volume
}

/// Result model:
///
/// ```json
/// { "error": "success", "data"?: "value" | 123 | true | null, "request_id"?: 123 }
/// ```
#[derive(Debug, Deserialize)]
pub struct MpvResponseResult {
	pub error: MpvResponseResultError,
	#[serde(default)]
	pub data: serde_json::Value,
	pub request_id: Option<i64>
}

#[derive(Debug, Deserialize)]
pub enum MpvResponseResultError {
	#[serde(rename = "success")]
	Success,
	#[serde(rename = "invalid parameter")]
	InvalidParameter
}

/// Either a mpv event or a mpv result.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum MpvResponse {
	Event(MpvResponseEvent),
	Result(MpvResponseResult)
}
