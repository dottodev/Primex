//! A versioned command bus, deliberately independent of JNI so it is fully
//! host-testable. The JNI layer in `crate` only converts strings and calls
//! [`CommandBus::dispatch`].

use pdx_core::{sha256_hex, ErrorKind, PdxError, Result, MIUIX_VERSION, TOOL_VERSION};
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;

type Handler = Box<dyn Fn(&str) -> Result<Response> + Send + Sync>;

/// Wire-level status of a response.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Ok,
    BadInput,
    Unsupported,
    Verification,
    Capability,
    Cancelled,
    Internal,
}

impl Status {
    /// Map a typed `PdxError` to its wire status. `Io` has no dedicated
    /// status, so it is reported as `Internal` (exit code 1).
    pub fn from_error(error: &PdxError) -> Self {
        match error.kind {
            ErrorKind::BadInput => Status::BadInput,
            ErrorKind::UnsupportedFormatVersion => Status::Unsupported,
            ErrorKind::VerificationFailed => Status::Verification,
            ErrorKind::CapabilityFailure => Status::Capability,
            ErrorKind::Cancelled => Status::Cancelled,
            ErrorKind::Io | ErrorKind::Internal => Status::Internal,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Status::Ok => "ok",
            Status::BadInput => "bad_input",
            Status::Unsupported => "unsupported",
            Status::Verification => "verification",
            Status::Capability => "capability",
            Status::Cancelled => "cancelled",
            Status::Internal => "internal",
        }
    }

    /// Exit-style numeric status derived from the underlying error kind.
    pub fn exit_code(self) -> i32 {
        match self {
            Status::Ok => 0,
            Status::BadInput => ErrorKind::BadInput.exit_code(),
            Status::Unsupported => ErrorKind::UnsupportedFormatVersion.exit_code(),
            Status::Verification => ErrorKind::VerificationFailed.exit_code(),
            Status::Capability => ErrorKind::CapabilityFailure.exit_code(),
            Status::Cancelled => ErrorKind::Cancelled.exit_code(),
            Status::Internal => ErrorKind::Internal.exit_code(),
        }
    }
}

/// Result of a command. Small data lives in `json`; large binary data lives
/// in `buffer` so it never has to be JSON-encoded.
#[derive(Debug, Clone)]
pub struct Response {
    pub status: Status,
    pub json: String,
    pub buffer: Option<Vec<u8>>,
}

impl Response {
    pub fn ok(value: Value) -> Self {
        Self {
            status: Status::Ok,
            json: value.to_string(),
            buffer: None,
        }
    }

    pub fn error(error: &PdxError) -> Self {
        Self {
            status: Status::from_error(error),
            json: json!({
                "kind": error.kind,
                "message": error.message,
            })
            .to_string(),
            buffer: None,
        }
    }

    pub fn with_buffer(mut self, buffer: Vec<u8>) -> Self {
        self.buffer = Some(buffer);
        self
    }

    /// Serialize the complete wire envelope, always including `request_id`.
    pub fn to_json(&self, request_id: u64) -> String {
        let data: Value =
            serde_json::from_str(&self.json).unwrap_or_else(|_| Value::String(self.json.clone()));
        let mut envelope = json!({
            "status": self.status.as_str(),
            "status_code": self.status.exit_code(),
            "request_id": request_id,
            "data": data,
        });
        if self.status != Status::Ok {
            if let Some(message) = data.get("message") {
                envelope["message"] = message.clone();
            }
        }
        if let Some(buffer) = &self.buffer {
            // Only the length crosses the JSON boundary; the bytes stay in
            // the `buffer` field of the Rust-side Response.
            envelope["buffer_len"] = json!(buffer.len());
        }
        envelope.to_string()
    }
}

/// Registry of named commands.
pub struct CommandBus {
    api_version: u16,
    handlers: HashMap<String, Handler>,
}

impl Default for CommandBus {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandBus {
    pub fn new() -> Self {
        Self {
            api_version: 1,
            handlers: HashMap::new(),
        }
    }

    /// A bus preloaded with the built-in commands.
    pub fn with_builtins() -> Self {
        let mut bus = Self::new();
        bus.register_builtins();
        bus
    }

    pub fn api_version(&self) -> u16 {
        self.api_version
    }

    pub fn register<F>(&mut self, id: impl Into<String>, handler: F)
    where
        F: Fn(&str) -> Result<Response> + Send + Sync + 'static,
    {
        self.handlers.insert(id.into(), Box::new(handler));
    }

    pub fn register_builtins(&mut self) {
        self.register("api.version", |_| Ok(Response::ok(json!({ "api": 1 }))));
        self.register("workspace.info", |_| {
            Ok(Response::ok(json!({
                "tool_version": TOOL_VERSION,
                "miuix_version": MIUIX_VERSION,
            })))
        });
        self.register("hash.sha256", |payload| {
            #[derive(Deserialize)]
            struct HashRequest {
                data: String,
            }
            let request: HashRequest = serde_json::from_str(payload).map_err(|error| {
                PdxError::bad_input(format!(
                    "hash.sha256 payload must be {{\"data\": string}}: {error}"
                ))
            })?;
            Ok(Response::ok(json!({
                "hex": sha256_hex(request.data.as_bytes()),
            })))
        });
    }

    /// Dispatch a command. Unknown commands return a typed `BadInput`.
    /// Successful responses carry the request id inside their JSON payload.
    pub fn dispatch(&self, request_id: u64, command: &str, payload: &str) -> Result<Response> {
        let handler = self.handlers.get(command).ok_or_else(|| {
            PdxError::bad_input("unknown command")
                .expected("registered command id", command.to_string())
        })?;
        let mut response = handler(payload)?;
        let mut value: Value = serde_json::from_str(&response.json).unwrap_or(Value::Null);
        if value.is_object() {
            if let Some(object) = value.as_object_mut() {
                object.insert("request_id".to_string(), json!(request_id));
            }
        } else {
            let previous = value;
            value = json!({ "request_id": request_id, "value": previous });
        }
        response.json = value.to_string();
        Ok(response)
    }

    /// Dispatch and produce a complete wire string. Errors are converted to
    /// a response so the request id and status always appear in the JSON.
    pub fn dispatch_json(&self, request_id: u64, command: &str, payload: &str) -> String {
        match self.dispatch(request_id, command, payload) {
            Ok(response) => response.to_json(request_id),
            Err(error) => Response::error(&error).to_json(request_id),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bus() -> CommandBus {
        CommandBus::with_builtins()
    }

    #[test]
    fn api_version_command() {
        let response = bus().dispatch(1, "api.version", "").unwrap();
        let value: Value = serde_json::from_str(&response.json).unwrap();
        assert_eq!(value["api"], 1);
        assert_eq!(value["request_id"], 1);
        assert_eq!(bus().api_version(), 1);
    }

    #[test]
    fn workspace_info_command() {
        let response = bus().dispatch(2, "workspace.info", "").unwrap();
        let value: Value = serde_json::from_str(&response.json).unwrap();
        assert_eq!(value["tool_version"], TOOL_VERSION);
        assert_eq!(value["miuix_version"], MIUIX_VERSION);
    }

    #[test]
    fn hash_sha256_known_vector() {
        let response = bus()
            .dispatch(3, "hash.sha256", r#"{"data":"abc"}"#)
            .unwrap();
        let value: Value = serde_json::from_str(&response.json).unwrap();
        assert_eq!(
            value["hex"],
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn unknown_command_is_bad_input() {
        let error = bus().dispatch(4, "nope", "").unwrap_err();
        assert_eq!(error.kind, ErrorKind::BadInput);
        let wire = bus().dispatch_json(4, "nope", "");
        let value: Value = serde_json::from_str(&wire).unwrap();
        assert_eq!(value["status"], "bad_input");
        assert_eq!(value["status_code"], 2);
        assert_eq!(value["request_id"], 4);
    }

    #[test]
    fn status_from_error_maps_every_kind() {
        assert_eq!(
            Status::from_error(&PdxError::bad_input("x")),
            Status::BadInput
        );
        assert_eq!(
            Status::from_error(&PdxError::unsupported("x")),
            Status::Unsupported
        );
        assert_eq!(
            Status::from_error(&PdxError::verification("x")),
            Status::Verification
        );
        assert_eq!(
            Status::from_error(&PdxError::capability("x")),
            Status::Capability
        );
        assert_eq!(
            Status::from_error(&PdxError::cancelled("x")),
            Status::Cancelled
        );
        assert_eq!(Status::from_error(&PdxError::io("x")), Status::Internal);
        assert_eq!(
            Status::from_error(&PdxError::internal("x")),
            Status::Internal
        );
    }

    #[test]
    fn status_exit_codes_match_contract() {
        assert_eq!(Status::Ok.exit_code(), 0);
        assert_eq!(Status::BadInput.exit_code(), 2);
        assert_eq!(Status::Unsupported.exit_code(), 3);
        assert_eq!(Status::Verification.exit_code(), 4);
        assert_eq!(Status::Capability.exit_code(), 5);
        assert_eq!(Status::Cancelled.exit_code(), 6);
        assert_eq!(Status::Internal.exit_code(), 1);
    }

    #[test]
    fn large_data_uses_buffer_not_json() {
        let mut bus = bus();
        bus.register("test.buffer", |_| {
            let mut payload = vec![0xABu8; 100_000];
            payload[0] = 1;
            Ok(Response::ok(json!({ "note": "large" })).with_buffer(payload))
        });
        let response = bus.dispatch(5, "test.buffer", "").unwrap();
        assert_eq!(response.buffer.as_ref().map(Vec::len), Some(100_000));
        assert!(response.json.len() < 128);
        let wire = response.to_json(5);
        assert!(wire.contains("\"buffer_len\":100000"));
        assert!(!wire.contains("171, 171"));
    }

    #[test]
    fn error_response_has_status_and_message() {
        let error = PdxError::capability("no hardware");
        let wire = Response::error(&error).to_json(7);
        let value: Value = serde_json::from_str(&wire).unwrap();
        assert_eq!(value["status"], "capability");
        assert_eq!(value["message"], "no hardware");
        assert_eq!(value["request_id"], 7);
    }

    #[test]
    fn handler_error_propagates_typed_error() {
        let mut bus = bus();
        bus.register("test.fail", |_| Err(PdxError::internal("boom")));
        let error = bus.dispatch(8, "test.fail", "").unwrap_err();
        assert_eq!(error.kind, ErrorKind::Internal);
        assert_eq!(error.message, "boom");
    }

    #[test]
    fn malformed_hash_payload_is_bad_input() {
        let error = bus().dispatch(9, "hash.sha256", "not json").unwrap_err();
        assert_eq!(error.kind, ErrorKind::BadInput);
    }
}
