mod ai_worker;
mod config;
// mod data_broker;
mod task_execution;

use std::{
    error::Error,
    fs,
    sync::{Arc, Mutex as SyncMutex},
};

use {
    log::{debug, error, info},
    percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC},
    tokio::{sync::Mutex, time::sleep},
};

use {
    ai_worker::{AIWorker, Message},
    config::Config,
    task_execution::{Scheduler, TaskManager},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    info!("Starting service");

    let config: Config = toml::from_str(include_str!("../config.toml"))?;

    let worker = Arc::new(SyncMutex::new(AIWorker::new(&config.model)?));
    let task_manager = Arc::new(Mutex::new(TaskManager::new().await?));

    {
        let task_manager = task_manager.lock().await;
        let mut scope = task_manager.scope.lock().unwrap();
        scope.insert::<Arc<SyncMutex<AIWorker>>>(worker);
    }

    {
        let mut task_manager = task_manager.lock().await;

        task_manager
            .register_function("llm_eval", |scope, params| {
                let mut llm = scope
                    .get_mut::<Arc<SyncMutex<AIWorker>>>()
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
            .register_function("http_get", |_scope, params| {
                debug!("Running http_get");
                if let Some(uri) = params.get("uri") {
                    if let Ok(uri) = serde_json::from_value::<String>(uri.clone()) {
                        debug!("Attempting to get uri: {}", uri);
                        // if let Some(_headers) = params.get("headers") {}

                        match ureq::get(&uri).call() {
                            Ok(result) => match result.into_string() {
                                Ok(body) => serde_json::from_str(&body).unwrap(),
                                Err(e) => {
                                    error!("Error in http_get: {}", e);
                                    serde_json::from_str(&format!("Error: {}", e)).unwrap()
                                }
                            },
                            Err(e) => {
                                error!("Error in http_get: {}", e);
                                serde_json::from_str(&format!("Error: {}", e)).unwrap()
                            }
                        }
                    } else {
                        serde_json::from_str("URI not of correct type").unwrap()
                    }
                } else {
                    serde_json::from_str("Message parameter not found").unwrap()
                }
            })
            .await
            .unwrap();

        task_manager
            .register_function("percent_encode", |_scope, params| {
                debug!("Running http_get");
                if let Some(input) = params.get("input") {
                    if let Ok(input) = serde_json::from_value::<String>(input.clone()) {
                        let encoded: String =
                            utf8_percent_encode(&input, NON_ALPHANUMERIC).to_string();
                        serde_json::from_str(&format!("{{\"output\": \"{}\"}}", &encoded)).unwrap()
                    } else {
                        serde_json::from_str("Input not of correct type").unwrap()
                    }
                } else {
                    serde_json::from_str("Input parameter not found").unwrap()
                }
            })
            .await
            .unwrap();

        task_manager
            .register_function("json_to_lua", |_scope, params| {
                if let Some(params) = params.get("params") {
                    match serde_json::from_value::<String>(params.clone()) {
                        Ok(value) => serde_json::from_str(&value).unwrap(),
                        Err(e) => serde_json::from_str(&format!("Error converting params: {}", e))
                            .unwrap(),
                    }
                } else {
                    serde_json::from_str("Params not provided").unwrap()
                }
            })
            .await
            .unwrap();
    }

    let mut scheduler = Scheduler::new().unwrap();

    for script in config.scripts.iter() {
        let script_contents = fs::read_to_string(script.path.to_str().unwrap()).unwrap();
        task_manager
            .lock()
            .await
            .register_script(&script_contents, script)
            .await
            .unwrap();
        for task in script.tasks.iter() {
            scheduler
                .register_task(task.name.clone(), task.cron.clone())
                .unwrap();
        }
    }

    loop {
        scheduler.run(task_manager.clone()).unwrap();
        sleep(std::time::Duration::from_millis(500)).await;
    }
}
