use std::path::PathBuf;

use {
    anyhow::{Context, Result},
    hf_hub::api::sync::ApiBuilder,
    serde::{Deserialize, Serialize},
};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub model: Model,
    pub scripts: Vec<Script>,
}

#[derive(Serialize, Deserialize)]
pub enum Model {
    Local { path: PathBuf },
    HuggingFace { repo: String, model: String },
}

impl Model {
    pub fn get_or_load(&self) -> Result<PathBuf> {
        match self {
            Model::Local { path } => Ok(path.clone()),
            Model::HuggingFace { model, repo } => ApiBuilder::new()
                .with_progress(true)
                .build()
                .with_context(|| "unable to create huggingface api")?
                .model(repo.clone())
                .get(model)
                .with_context(|| "unable to download model"),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Script {
    pub path: PathBuf,
    pub tasks: Vec<Task>,
}

#[derive(Serialize, Deserialize)]
pub struct Task {
    pub name: String,
    pub cron: String,
}
