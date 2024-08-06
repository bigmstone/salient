mod ai_worker;
// mod data_broker;
mod event_pipeline;
mod task_execution;

use std::{
    error::Error,
    sync::{Arc, Mutex},
};

use log::info;

use {
    ai_worker::{llm::Message, AIWorker},
    task_execution::{Task, TaskManager},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    info!("Starting service");

    let worker = Arc::new(Mutex::new(AIWorker::new()?));
    let mut task_manager = TaskManager::new().await?;

    {
        let mut scope = task_manager.scope.lock().unwrap();
        scope.insert::<Arc<Mutex<AIWorker>>>(worker);
    }

    task_manager
        .register_function("llm_eval", |scope, params| {
            let mut llm = scope
                .get_mut::<Arc<Mutex<AIWorker>>>()
                .unwrap()
                .lock()
                .unwrap();

            if let Some(messages) = params.get("messages") {
                if let Ok(messages) = serde_json::from_value::<Vec<Message>>(messages.clone()) {
                    let messages = messages.to_owned();
                    let result = llm.eval(&messages).unwrap();

                    serde_json::to_value(result).unwrap()
                } else {
                    serde_json::from_str("Messages were not in correct format.").unwrap()
                }
            } else {
                serde_json::from_str("Message parameter not found").unwrap()
            }
        })
        .await
        .unwrap();

    task_manager
        .schedule(Task {
            task_name: String::from("Eval"),
            params: String::from(""),
        })
        .await?;

    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
}
