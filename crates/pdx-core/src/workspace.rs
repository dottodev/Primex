//! Schema-versioned workspace documents (`.pdxproj/project.json`, `profile.json`).
//!
//! Every JSON document carries `schema_version`. A migration is tested before
//! it is accepted; an unknown version is a typed `UnsupportedFormatVersion`.

use crate::error::{PdxError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PROJECT_SCHEMA_VERSION: u32 = 1;
pub const PROFILE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectJson {
    pub schema_version: u32,
    pub name: String,
    pub target_package: String,
    #[serde(default)]
    pub game_version: Option<String>,
    #[serde(default)]
    pub apk_version_code: Option<u64>,
    pub tool_version: String,
    pub miuix_version: String,
    pub capability_level: u8,
}

impl ProjectJson {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version == 0 || self.schema_version > PROJECT_SCHEMA_VERSION {
            return Err(
                PdxError::unsupported("project.json schema version").expected(
                    format!("1..={PROJECT_SCHEMA_VERSION}"),
                    self.schema_version.to_string(),
                ),
            );
        }
        if self.name.trim().is_empty() {
            return Err(
                PdxError::bad_input("project name is empty").expected("non-empty name", "empty")
            );
        }
        if self.target_package.trim().is_empty() {
            return Err(PdxError::bad_input("target_package is empty")
                .expected("reverse-DNS package", "empty"));
        }
        if self.capability_level > 3 {
            return Err(PdxError::bad_input("capability_level out of range")
                .expected("0..=3", self.capability_level.to_string()));
        }
        Ok(())
    }

    pub fn from_json(input: &str) -> Result<Self> {
        let value: Value = serde_json::from_str(input)
            .map_err(|e| PdxError::bad_input(format!("project.json is not valid JSON: {e}")))?;
        let project: Self = serde_json::from_value(migrate_project(value)?)
            .map_err(|e| PdxError::bad_input(format!("project.json does not match schema: {e}")))?;
        project.validate()?;
        Ok(project)
    }

    pub fn to_json_pretty(&self) -> Result<String> {
        self.validate()?;
        serde_json::to_string_pretty(self)
            .map_err(|e| PdxError::internal(format!("cannot serialize project.json: {e}")))
    }
}

/// Migrates historical documents forward. v0 had no `schema_version` field.
pub fn migrate_project(mut value: Value) -> Result<Value> {
    let version = value
        .get("schema_version")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    if version > PROJECT_SCHEMA_VERSION as u64 {
        return Err(PdxError::unsupported("project.json schema version")
            .expected(format!("<= {PROJECT_SCHEMA_VERSION}"), version.to_string()));
    }
    if version == 0 {
        let object = value
            .as_object_mut()
            .ok_or_else(|| PdxError::bad_input("project.json root must be an object"))?;
        object.insert(
            "schema_version".to_string(),
            Value::Number(PROJECT_SCHEMA_VERSION.into()),
        );
        object
            .entry("tool_version".to_string())
            .or_insert_with(|| Value::String(crate::TOOL_VERSION.to_string()));
        object
            .entry("miuix_version".to_string())
            .or_insert_with(|| Value::String(crate::MIUIX_VERSION.to_string()));
        object
            .entry("capability_level".to_string())
            .or_insert_with(|| Value::Number(0.into()));
    }
    Ok(value)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProfileJson {
    pub schema_version: u32,
    #[serde(default)]
    pub parser_versions: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    pub format_assumptions: std::collections::BTreeMap<String, String>,
    /// Named MLBB layout roots. Never hardcoded in a parser.
    #[serde(default)]
    pub path_roots: std::collections::BTreeMap<String, String>,
}

impl ProfileJson {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version == 0 || self.schema_version > PROFILE_SCHEMA_VERSION {
            return Err(
                PdxError::unsupported("profile.json schema version").expected(
                    format!("1..={PROFILE_SCHEMA_VERSION}"),
                    self.schema_version.to_string(),
                ),
            );
        }
        Ok(())
    }

    pub fn from_json(input: &str) -> Result<Self> {
        let value: Value = serde_json::from_str(input)
            .map_err(|e| PdxError::bad_input(format!("profile.json is not valid JSON: {e}")))?;
        let profile: Self = serde_json::from_value(value)
            .map_err(|e| PdxError::bad_input(format!("profile.json does not match schema: {e}")))?;
        profile.validate()?;
        Ok(profile)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_project() -> &'static str {
        r#"{
            "schema_version": 1,
            "name": "MLBB mod audit",
            "target_package": "com.example.mod",
            "tool_version": "0.1.0",
            "miuix_version": "0.9.4",
            "capability_level": 0
        }"#
    }

    #[test]
    fn valid_project_parses() {
        let p = ProjectJson::from_json(valid_project()).unwrap();
        assert_eq!(p.capability_level, 0);
        assert!(p.to_json_pretty().is_ok());
    }

    #[test]
    fn v0_document_migrates() {
        let input = r#"{"name":"x","target_package":"com.x"}"#;
        let p = ProjectJson::from_json(input).unwrap();
        assert_eq!(p.schema_version, 1);
        assert_eq!(p.tool_version, crate::TOOL_VERSION);
    }

    #[test]
    fn future_version_is_typed() {
        let input = valid_project().replace("\"schema_version\": 1", "\"schema_version\": 99");
        let err = ProjectJson::from_json(&input).unwrap_err();
        assert_eq!(err.kind, crate::error::ErrorKind::UnsupportedFormatVersion);
    }

    #[test]
    fn bad_capability_is_typed() {
        let input = valid_project().replace("\"capability_level\": 0", "\"capability_level\": 9");
        let err = ProjectJson::from_json(&input).unwrap_err();
        assert_eq!(err.kind, crate::error::ErrorKind::BadInput);
    }
}
