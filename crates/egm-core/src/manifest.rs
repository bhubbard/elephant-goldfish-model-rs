// Copyright 2026 Google LLC.
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ManifestError {
    #[error("Failed to read manifest at {0}: {1}")]
    IoError(String, #[source] std::io::Error),
    #[error("Failed to parse manifest JSON at {0}: {1}")]
    JsonError(String, #[source] serde_json::Error),
    #[error("Manifest validation error: {0}")]
    ValidationError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub author: Option<AuthorInfo>,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub hooks: Option<serde_json::Value>,
    #[serde(default)]
    pub apps: Option<serde_json::Value>,
    #[serde(default)]
    pub mcp_servers: Option<serde_json::Value>,
    #[serde(default)]
    pub mcp: Option<serde_json::Value>,
    #[serde(default)]
    pub extensions: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum AuthorInfo {
    String(String),
    Object {
        name: String,
        #[serde(default)]
        email: Option<String>,
        #[serde(default)]
        url: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClaudePluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub author: Option<AuthorInfo>,
    #[serde(default)]
    pub repository: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub agents: Option<serde_json::Value>,
    #[serde(default)]
    pub hooks: Option<serde_json::Value>,
    #[serde(default)]
    pub mcp_servers: Option<serde_json::Value>,
    #[serde(default)]
    pub mcp: Option<serde_json::Value>,
    #[serde(default)]
    pub apps: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CodexPluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub interface: Option<CodexInterface>,
    #[serde(default)]
    pub hooks: Option<serde_json::Value>,
    #[serde(default)]
    pub mcp_servers: Option<serde_json::Value>,
    #[serde(default)]
    pub mcp: Option<serde_json::Value>,
    #[serde(default)]
    pub apps: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CodexInterface {
    #[serde(default)]
    pub composer_icon: Option<String>,
    #[serde(default)]
    pub logo: Option<String>,
}

impl PluginManifest {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ManifestError> {
        let p = path.as_ref();
        let content = std::fs::read_to_string(p)
            .map_err(|e| ManifestError::IoError(p.display().to_string(), e))?;
        serde_json::from_str(&content)
            .map_err(|e| ManifestError::JsonError(p.display().to_string(), e))
    }
}

impl ClaudePluginManifest {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ManifestError> {
        let p = path.as_ref();
        let content = std::fs::read_to_string(p)
            .map_err(|e| ManifestError::IoError(p.display().to_string(), e))?;
        serde_json::from_str(&content)
            .map_err(|e| ManifestError::JsonError(p.display().to_string(), e))
    }
}

impl CodexPluginManifest {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, ManifestError> {
        let p = path.as_ref();
        let content = std::fs::read_to_string(p)
            .map_err(|e| ManifestError::IoError(p.display().to_string(), e))?;
        serde_json::from_str(&content)
            .map_err(|e| ManifestError::JsonError(p.display().to_string(), e))
    }
}
