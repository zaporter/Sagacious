use crate::config;
use crate::config::Config;
use serde::{Deserialize, Serialize};

use async_openai::types::*;
use async_openai::Client;
use std::{error::Error, fs::read_to_string, path::PathBuf};

// To be saved and read as JSON
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KnowledgeSnippets {
    config: Config,
    files: Vec<SourceFile>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceFile {
    path: PathBuf,
    content: String,
    line_count: usize,
    sections: Vec<SourceSection>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SourceSection {
    start_line: usize,
    end_line: usize,
    embedding_vector: Vec<f32>,
}

impl KnowledgeSnippets {
    // TODO:
    // Speed this up by using async to join all of the futures and do all embedding at once
    pub async fn new(config: Config, client: &Client) -> anyhow::Result<Self> {
        let files = config.get_source_files();
        let mut source_files = vec![];
        let bar = indicatif::ProgressBar::new(files.len() as u64);
        for file in files {
            bar.println(format!("Processing {}", file.display()));
            let source = SourceFile::from_path(file, &config, client).await?;
            source_files.push(source);
            bar.inc(1);
        }
        bar.finish();

        Ok(Self {
            config,
            files: source_files,
        })

    }
}
impl SourceFile {
    pub async fn from_path(path: PathBuf, config: &Config, client: &Client) -> anyhow::Result<Self> {
        let source = read_to_string(&path)?;
        let mut sections = vec![];

        let lines = source.lines().collect::<Vec<&str>>();
        let line_count = lines.len();

        for i in (0..line_count).step_by(config.section_subdividing_offset_lines) {
            let end = std::cmp::min(i + config.section_subdividing_size_lines, line_count);
            let split: String = lines[i..end].join("\n");

            // https://openai.com/blog/introducing-text-and-code-embeddings/
            let request = CreateEmbeddingRequestArgs::default()
                .model(&config.code_search_embedding_model)
                .input([&split])
                .build()?;

            let response = client.embeddings().create(request).await?;

            let response = response
                .data
                .get(0)
                .expect("Code embedding did not return a vector");
            let embedding = response.embedding.clone();
            sections.push(SourceSection {
                start_line: i,
                end_line: end,
                embedding_vector: embedding,
            });
        }
        Ok(Self {
            path,
            content: source,
            line_count,
            sections,
        })
    }
    pub fn read_lines(&self, start_line: usize, end_line: usize) -> String {
        let end_line = std::cmp::min(end_line, self.line_count - 1);
        let start_line = std::cmp::min(start_line, self.line_count - 1);
        return self
            .content
            .lines()
            .skip(start_line)
            .take(1 + end_line - start_line)
            .collect::<Vec<&str>>()
            .join("\n");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn read_lines_test() {
        let sf = SourceFile {
            path: PathBuf::from("."),
            content: r#"Hello
my name is blank
and I am a big fan rust
and its cool 
type system"#
                .into(),
            line_count: 5,
            sections: vec![],
        };
        assert_eq!(
            sf.read_lines(0, 2),
            r#"Hello
my name is blank
and I am a big fan rust"#
        );
        assert_eq!(sf.read_lines(5, 6), r#"type system"#);
    }
}
