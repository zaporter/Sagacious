use std::{error::Error, fs::read_to_string, path::PathBuf};

use async_openai::types::*;
use async_openai::{types::CreateCompletionRequestArgs, types::CreateEditRequestArgs, Client};
use serde::{Deserialize, Serialize};

use clap::{Parser, Subcommand};

mod config;
mod documentation;

use config::Config;

use crate::documentation::KnowledgeSnippets;

extern crate boilerplate;


#[derive(boilerplate::Boilerplate)]
#[boilerplate(filename = "readme-prompt.txt")]
struct ReadmePrompt {
    filename: String,
    filecontents: String,
}

#[derive(Parser)]
#[command(author, version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands
}

#[derive(Subcommand)]
#[non_exhaustive]
enum Commands {
    /// Generate the documentation
    Gen {
        /// Path to config file
        #[arg(short, long, default_value="./RepoGPT/config.toml")]
        config: PathBuf, 
        /// Output directory for files
        #[arg(short,long, default_value="./RepoGPT")]
        out: PathBuf,
        /// Open the documentation after it is created
        #[arg(long)]
        open : bool
    },
    Open {

    },
    /// Create a default config file as a starting point
    CreateConfig {
        /// Where to write this file
        #[arg(short, long, default_value="./RepoGPT/config.toml")]
        path: PathBuf,
    },




}

#[tokio::main]
async fn main() -> anyhow::Result<()>{
    env_logger::init_from_env(env_logger::Env::new().default_filter_or("info"));
    let cli = Cli::parse();
    match cli.command {

        Commands::CreateConfig{path}=> {
            log::info!("Creating a config at {}", &path.to_str().unwrap());
            let toml = toml::to_string(&Config::default()).unwrap();
            std::fs::create_dir_all(path.parent().unwrap_or(&PathBuf::from("."))).unwrap();
            std::fs::write(path,toml).unwrap();
        },
        Commands::Gen{config, mut out, open}=> {
            log::info!("Starting Source Generation");
            let config = Config::from_path(&config)?;
            

            let client = Client::new();
            let knowledge = KnowledgeSnippets::new(config, &client).await?;

            out.push("knowledge_snippets.json");
            std::fs::create_dir_all(out.parent().unwrap_or(&PathBuf::from("."))).unwrap();
            let knowledge = serde_json::to_string(&knowledge)?;
            std::fs::write(out,knowledge)?;



        },
        _=> {
            todo!();
        }

    }
    // let client = Client::new();

    // let path = PathBuf::from("./examples/gml_parser/");
    // let mut readme = path.clone();
    // readme.push("README.md");
    // let readme_contents = read_to_string(readme)?;
    // let readme_prompt = ReadmePrompt {
    //     filename: "README.md".into(),
    //     filecontents: readme_contents.clone().into(),
    // };
    // println!("{}", readme_prompt.to_string());

    // // single
    // let request = CreateCompletionRequestArgs::default()
    //     .model("text-davinci-003")
    //     .prompt(readme_prompt.to_string())
    //     .max_tokens(120_u16)
    //     .build()?;

    // let response = client.completions().create(request).await?;

    // println!("\nResponse (single):\n");
    // for choice in response.choices {
    //     println!("{}", choice.text);
    // }
    // let request = CreateEditRequestArgs::default()
    //        .model("text-davinci-edit-001")
    //        .input(concat!(
    //            "It's surely our responsibility to do everything within our power ",
    //            "to create a planet that provides a home not just for us, ",
    //            "but for all life on Earth."
    //        ))
    //        .instruction("Add a new paragraph in Sir David Attenborough voice")
    //        .n(2)
    //        .temperature(0.9)
    //        .build()?;

    //    let response = client.edits().create(request).await?;

    //    for choice in response.choices {
    //        println!("{} \n----", choice.text)


    Ok(())
}

async fn create_file_sections(
    file_contents: &str,
    llm_client: &Client,
) -> anyhow::Result<Vec<String>> {
    let break_prompt = read_to_string("./templates/break-prompt.txt")?;
    let client = Client::new();
    let lines: Vec<&str> = file_contents.lines().collect();
    let mut current_min_line = 0;
    let num_lines = lines.len();
    let section_lines = 100;
    let last_section_size = 50;
    let mut result = Vec::new();

    while num_lines - current_min_line > last_section_size {
        let end_line = current_min_line + section_lines.min(num_lines - current_min_line);
        let current_uncut_section = &lines[current_min_line..end_line];
        let current_uncut_section = current_uncut_section.join("\n");
        println!("curr:{}", &current_uncut_section);
        println!("{}", &break_prompt);
        println!();
        println!();
        let request = CreateEditRequestArgs::default()
            .model("code-davinci-edit-001")
            .input(current_uncut_section)
            .instruction(&break_prompt)
            // .n(10)
            .temperature(0.9)
            .build()?;

        let response = client.edits().create(request).await?;
        dbg!(&response.choices);
        let response = response
            .choices
            .get(0)
            .expect("No choice returned for sections query");
        let mut sections = response.text.split("--SECTION--BREAK--").peekable();
        while let Some(section) = sections.next() {
            // Ignore the last section. It is better to have the LLM do all of the splits.
            if sections.peek().is_some() {
                let section_lines = section.lines().count();
                // We reference the real source file rather than using the section content
                // in case the LLM decided to also edit the source
                let real_section =
                    lines[current_min_line..(current_min_line + section_lines)].join("\n");
                println!("SUCCESS SUCCESS SUCCESS -------------------------------------------");
                println!("SUCCESS SUCCESS SUCCESS -------------------------------------------");
                dbg!(&real_section);
                println!("END SUCCESS SUCCESS -------------------------------------------");
                result.push(real_section);
                current_min_line += section_lines;
            }
        }
    }

    result.push(lines[current_min_line..num_lines].join("\n"));
    Ok(result)
}
