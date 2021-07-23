use serde::Deserialize;

/// Event model:
///
/// ```json
/// { "event": "idle" }
/// { "event": "property-change", id: number, name: "property-name", data: "property-value" }
/// ```
#[derive(Debug, Deserialize)]
#[serde(tag = "event")]
pub enum MpvResponseEvent {
	#[serde(rename = "idle")]
	Idle,
	#[serde(rename = "property-change")]
	PropertyChange {
		/// Id of the observer.
		id: i64,
		name: MpvResponseEventPropertyName,
		data: serde_json::Value
	}
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
