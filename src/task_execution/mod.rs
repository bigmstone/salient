use std::{error::Error, sync::Arc};

use {
    mlua::prelude::*,
    tokio::sync::{mpsc, Mutex},
};

pub struct Task {
    pub task_name: String,
    pub params: String,
}

pub struct TaskManager {
    tasks: Vec<String>,
    tx: mpsc::Sender<Task>,
}

impl TaskManager {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let (tx, mut rx): (mpsc::Sender<Task>, mpsc::Receiver<Task>) = mpsc::channel(100);

        let lua = Arc::new(Mutex::new(Lua::new()));

        let task_lua = lua.clone();

        tokio::spawn(async move {
            while let Some(task) = rx.recv().await {
                task_lua
                    .lock()
                    .await
                    .load(&format!("{}.execute({})", task.task_name, task.params))
                    .exec()
                    .expect("");
            }
        });

        let lua = lua.lock().await;

        let mut scripts = [(
            vec![String::from("Daily"), String::from("Weekly")],
            include_str!("./scripts/daily.lua"),
        )];

        let mut tasks = vec![];

        for (local_tasks, script) in scripts.iter_mut() {
            lua.load(script.to_owned()).exec()?;

            for task in local_tasks {
                lua.load(&format!("{}.setup()", task)).exec()?;
                tasks.push(task.clone());
            }
        }

        Ok(Self { tasks, tx })
    }

    pub async fn schedule(&mut self, task: Task) -> Result<(), Box<dyn Error>> {
        if !self.tasks.iter().any(|i| i == &task.task_name) {
            panic!("No task");
        }
        self.tx.send(task).await?;
        Ok(())
    }
}

// let script = script.lock().await;
// if let Some(table_name) = script.script_map.get(&event.key) {
//     let method = match event.action {
//         Action::Press => format!("{}.Press", table_name),
//         Action::Release => format!("{}.Release", table_name),
//     };

//     trace!("Executing script: {}", method);

//     script.lua.load(&format!("{}()", method)).exec().unwrap();
// }
