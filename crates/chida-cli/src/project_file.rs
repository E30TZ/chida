//! The on-disk `.chida.json` bundle: design, data and logic in one file.
use chida_core::Project;
use chida_db::Database;
use chida_logic::Flow;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFile {
    pub project: Project,
    #[serde(default)]
    pub database: Database,
    #[serde(default)]
    pub flows: Vec<Flow>,
}
impl ProjectFile {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        std::fs::write(path, self.to_json().map_err(std::io::Error::other)?)
    }
    pub fn load(path: &std::path::Path) -> std::io::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Self::from_json(&text).map_err(std::io::Error::other)
    }
    pub fn validate(&self) -> Vec<String> {
        let mut issues = self.project.validate();
        issues.extend(self.database.validate());
        issues
    }
}
