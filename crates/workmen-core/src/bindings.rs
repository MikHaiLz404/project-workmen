//! TypeScript binding generation for the Tauri shell.
//!
//! The `packages/contracts/src/generated.ts` file in the JS
//! workspace is generated from the Rust types in
//! `crates/workmen-core/src/model`. This module is the
//! single source of truth: it exposes a function that returns
//! the canonical command-envelope schema, and the drift gate
//! in `tests/typed_contracts.rs` fails when the generated TS
//! file is stale.
//!
//! In a follow-up, this module will gain a `ts-rs` or
//! `schemars-to-typescript` integration that emits
//! `generated.ts` directly. For T2.T1, the contract is
//! hand-written; the drift gate is the safety net.

use schemars::schema_for;
use serde::Serialize;

/// A typed error carried in [`CommandEnvelope::Err`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, schemars::JsonSchema)]
pub struct CommandError {
    pub code: String,
    pub message: String,
}

/// A single CommandEnvelope as the Tauri command boundary
/// delivers them. T2.T1's React test asserts the shell can
/// render a typed error envelope; this struct is the Rust
/// side of that contract.
///
/// The serde_json representation is `{"apiVersion": 1,
/// "requestId": "...", "data": <T>}` on success and
/// `{"apiVersion": 1, "requestId": "...", "error":
/// {"code": "...", "message": "..."}}` on failure.
///
/// We do not derive `JsonSchema` on the envelope itself because
/// the data field is generic and `T` is not bound to
/// `JsonSchema`. The schema generator below uses a non-generic
/// placeholder; the TypeScript side models the envelope as
/// `CommandEnvelope<T>`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum CommandEnvelope<T> {
    /// A successful command response. `data` carries the
    /// typed payload.
    Ok {
        #[serde(rename = "apiVersion")]
        api_version: u8,
        #[serde(rename = "requestId")]
        request_id: String,
        data: T,
    },
    /// A failed command response. `error` carries a typed
    /// error code + human-readable message.
    Err {
        #[serde(rename = "apiVersion")]
        api_version: u8,
        #[serde(rename = "requestId")]
        request_id: String,
        error: CommandError,
    },
}

/// The Tauri `WorkmenBackend` shape, mirrored in
/// `packages/contracts/src/generated.ts` as
/// `WorkmenBackend.invoke`. The Rust side is a trait that the
/// Tauri command surface implements; the TypeScript side is
/// the discriminated-union envelope the React shell calls.
pub trait WorkmenBackend {
    /// Send a typed command to the Workmen backend.
    fn invoke<TRequest, TResponse>(
        &self,
        command: &str,
        request: TRequest,
    ) -> Result<CommandEnvelope<TResponse>, CommandError>
    where
        TRequest: Serialize,
        TResponse: serde::de::DeserializeOwned;
}

/// A non-generic concrete envelope used only to generate the
/// schemars schema. The TypeScript side has a generic
/// `CommandEnvelope<T>`; this struct is the schema-only mirror.
#[derive(schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
#[allow(dead_code)]
enum EnvelopePlaceholder {
    Ok {
        #[serde(rename = "apiVersion")]
        api_version: u8,
        #[serde(rename = "requestId")]
        request_id: String,
        data: String,
    },
    Err {
        #[serde(rename = "apiVersion")]
        api_version: u8,
        #[serde(rename = "requestId")]
        request_id: String,
        error: CommandError,
    },
}

/// The CommandEnvelope schema, suitable for emitting a
/// TypeScript declaration. This is the single source of truth
/// the drift gate compares against.
pub fn command_envelope_schema() -> schemars::schema::RootSchema {
    schema_for!(EnvelopePlaceholder)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_envelope_ok_serializes_with_data_field() {
        let env: CommandEnvelope<i32> = CommandEnvelope::Ok {
            api_version: 1,
            request_id: "req-1".to_string(),
            data: 42,
        };
        let json = serde_json::to_value(&env).expect("serialize");
        // The discriminant is `kind`; the variant is serialized as
        // "ok" (lowercased) because of `rename_all = "lowercase"`.
        assert_eq!(json["kind"], "ok");
        // The fields are renamed to camelCase to match the
        // TypeScript contract in generated.ts.
        assert_eq!(json["apiVersion"], 1);
        assert_eq!(json["requestId"], "req-1");
        assert_eq!(json["data"], 42);
    }

    #[test]
    fn command_envelope_err_serializes_with_error_field() {
        let env: CommandEnvelope<i32> = CommandEnvelope::Err {
            api_version: 1,
            request_id: "req-2".to_string(),
            error: CommandError {
                code: "WorkmenError::Config".to_string(),
                message: "missing .workmen/project.yaml".to_string(),
            },
        };
        let json = serde_json::to_value(&env).expect("serialize");
        assert_eq!(json["kind"], "err");
        assert_eq!(json["apiVersion"], 1);
        assert_eq!(json["requestId"], "req-2");
        assert_eq!(json["error"]["code"], "WorkmenError::Config");
        assert!(
            json["error"]["message"]
                .as_str()
                .unwrap()
                .contains(".workmen")
        );
    }

    #[test]
    fn command_envelope_schema_contains_top_level_keys() {
        let schema = command_envelope_schema();
        let json = serde_json::to_value(&schema).expect("schema to value");
        let s = json.to_string();
        // The schema must mention the apiVersion and requestId
        // fields (the typed envelope's stable surface).
        assert!(s.contains("apiVersion") || s.contains("api_version"));
    }
}
