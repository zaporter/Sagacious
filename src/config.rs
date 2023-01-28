use std::{fs::FileType, path::{PathBuf, Path}};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

// To be saved and read as TOML
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Config {
    pub config_version: u32,
    pub source_files: Vec<String>,
    pub provided_facts: Vec<String>,
    pub doc_generating_model: String,
    pub query_executor_model: String,
    pub edit_model: String,
    pub code_search_embedding_model: String,
    pub budget: f64,
    pub section_subdividing_size_lines: usize,
    pub section_subdividing_offset_lines: usize,
}
impl Default for Config {
    fn default() -> Self {
        Config {
            config_version: 1,
            source_files: vec!["./README.md".into(), "./*.txt".into(), "./*.rs".into(), "./src/".into()],
            provided_facts: vec![],
            doc_generating_model: "text-davinci-003".into(),
            query_executor_model: "text-davinci-003".into(),
            edit_model: "text-davinci-edit-001".into(),
            // Good options:
            // text-search-davinci-query-001
            // text-embedding-ada-002
            // code-search-babbage-code-001
            code_search_embedding_model: "text-embedding-ada-002".into(),
            budget: 10.0,
            section_subdividing_size_lines: 50,
            section_subdividing_offset_lines: 25,
        }
    }
}
impl Config {
    pub fn from_path(path: &PathBuf) -> anyhow::Result<Self> {
        let config_contents = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&config_contents)?;
        if config.config_version != Self::default().config_version {
            anyhow::bail!("Config version is incorrect for this version of the program.")
        }
        Ok(config)
    }
    pub fn get_source_files(&self) -> Vec<PathBuf> {
        let mut results = vec![];
        let mut patterns = Vec::new();
        for str_pattern in &self.source_files {
            patterns.push(regex::Regex::new(&str_pattern).unwrap())
        }
        'file: for entry in WalkDir::new(".").into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() {
                for pattern in &patterns {
                    if pattern.is_match(&entry.path().to_string_lossy()) {
                        results.push(PathBuf::from(entry.path()));
                        continue 'file;
                    }
                }
            }
        }
        results
    }
}
