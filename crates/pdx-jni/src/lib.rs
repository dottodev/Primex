//! pdx-jni — JNI bridge to the versioned command bus.
//!
//! The bus itself (`crate::bus`) is JNI-independent and host-testable. This
//! module only converts Java strings to Rust and back; it never throws and
//! never panics across the JNI boundary.

pub mod bus;
pub use bus::{CommandBus, Response, Status};

use jni::objects::{JClass, JString};
use jni::JNIEnv;
use pdx_core::{PdxError, Result};
use std::sync::OnceLock;

static BUS: OnceLock<CommandBus> = OnceLock::new();

fn global_bus() -> &'static CommandBus {
    BUS.get_or_init(CommandBus::with_builtins)
}

/// Single JNI entry point. The `JString` returned is always a JSON object
/// containing `status`, `status_code`, `request_id` and either `data` or a
/// top-level `message` on failure.
#[no_mangle]
pub extern "system" fn Java_dev_primex_dt_jni_CommandBus_nativeSend<'local>(
    mut env: JNIEnv<'local>,
    _class: JClass<'local>,
    command: JString<'local>,
    payload: JString<'local>,
) -> JString<'local> {
    let outcome: Result<String> = (|| {
        let command_java = env
            .get_string(&command)
            .map_err(|error| PdxError::internal(format!("JNI command decode failed: {error}")))?;
        let payload_java = env
            .get_string(&payload)
            .map_err(|error| PdxError::internal(format!("JNI payload decode failed: {error}")))?;
        let command_text: String = command_java.into();
        let payload_text: String = payload_java.into();
        Ok(global_bus().dispatch_json(0, &command_text, &payload_text))
    })();

    let text = match outcome {
        Ok(text) => text,
        Err(error) => Response::error(&error).to_json(0),
    };

    match env.new_string(text) {
        Ok(result) => result,
        // A pending JNI exception means no string can be created; returning a
        // null JString is the only non-panicking option here.
        Err(_) => unsafe { JString::from_raw(std::ptr::null_mut()) },
    }
}
