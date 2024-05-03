mod ai_worker;
mod data_broker;
mod event_pipeline;
mod task_execution;

use std::error::Error;

use log::info;

use {
    ai_worker::AIWorker,
    task_execution::{Task, TaskManager},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    info!("Starting service");

    let mut worker = AIWorker::new()?;
    let mut task_manager = TaskManager::new().await?;

    task_manager
        .schedule(Task {
            task_name: String::from("Daily"),
            params: String::from(""),
        })
        .await?;

    task_manager
        .schedule(Task {
            task_name: String::from("Weekly"),
            params: String::from(""),
        })
        .await?;

    worker
        .eval(&[
            ai_worker::llm::Message::new("system", "You are Leroy, a helpful AI assistant."),
            ai_worker::llm::Message::new(
                "system",
                "We need to get the latest weather for Matthew. He is curently in Ruston, Louisiana.",
            ),
        ])
        .await?;

    Ok(())
}
