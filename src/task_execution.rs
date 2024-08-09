use std::{
    any::{Any, TypeId},
    collections::HashMap,
    env,
    error::Error,
    str::FromStr,
    sync::{Arc, Mutex as StdMutex},
    time::Duration,
};

use {
    anyhow::Result,
    chrono::Utc,
    cron::Schedule,
    log::{debug, error},
    mlua::{prelude::*, LuaSerdeExt},
    serde_json::Value as JsonValue,
    tokio::{sync::Mutex, task::JoinHandle, time::sleep},
};

use crate::config::Script;

const PATH_ENV_NAME: &str = "LUA_PATH";

pub struct Scope {
    map: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    pub fn insert<T: 'static + Any + Send + Sync>(&mut self, item: T) {
        let type_id = TypeId::of::<T>();
        self.map.insert(type_id, Box::new(item));
    }

    pub fn _get<T: 'static + Any + Send + Sync>(&self) -> Option<&T> {
        let type_id = TypeId::of::<T>();
        self.map
            .get(&type_id)
            .and_then(|item| item.downcast_ref::<T>())
    }

    pub fn get_mut<T: 'static + Any + Send + Sync>(&mut self) -> Option<&mut T> {
        let type_id = TypeId::of::<T>();
        self.map
            .get_mut(&type_id)
            .and_then(|item| item.downcast_mut::<T>())
    }
}

pub struct Task {
    pub task_name: String,
    pub params: String,
}

pub struct TaskManager {
    lua: Arc<Mutex<Lua>>,
    pub scope: Arc<StdMutex<Scope>>,
    tasks: Vec<String>,
    lua_path: String,
}

impl TaskManager {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let lua = Arc::new(Mutex::new(unsafe { Lua::unsafe_new() }));

        Ok(Self {
            lua,
            tasks: vec![],
            scope: Arc::new(StdMutex::new(Scope::new())),
            lua_path: env::var(PATH_ENV_NAME).unwrap_or(String::from("")),
        })
    }

    pub async fn register_script(
        &mut self,
        contents: &str,
        script: &Script,
    ) -> Result<(), Box<dyn Error>> {
        let lua = self.lua.lock().await;

        // let mut scripts = [(
        //     vec![String::from("Eval")],
        //     include_str!("./scripts/script.lua"),
        // )];

        lua.load(contents).exec()?;

        env::set_var(
            PATH_ENV_NAME,
            format!(
                "{};{}/?.lua",
                self.lua_path,
                script.path.parent().unwrap().to_str().unwrap()
            ),
        );

        for task in &script.tasks {
            lua.load(&format!("{}.setup()", task.name)).exec()?;
            self.tasks.push(task.name.clone());
        }

        Ok(())
    }

    pub async fn register_function<F>(&mut self, name: &str, function: F) -> Result<()>
    where
        F: Fn(&mut Scope, JsonValue) -> JsonValue + Send + Sync + 'static,
    {
        let lua = self.lua.lock().await;
        let function = Arc::new(function);
        let scope = self.scope.clone();

        let lua_function = lua.create_function(move |lua_ctx, params: mlua::Value| {
            let json_params: JsonValue = lua_ctx.from_value(params)?;
            let mut scope = scope.lock().unwrap();
            let result = function(&mut scope, json_params);
            lua_ctx.to_value(&result).map_err(LuaError::external)
        })?;

        lua.globals().set(name, lua_function).unwrap();

        Ok(())
    }

    pub async fn schedule(&mut self, task: Task) -> Result<(), Box<dyn Error>> {
        if !self.tasks.iter().any(|i| i == &task.task_name) {
            panic!("No task");
        }

        let task_lua = self.lua.clone();
        tokio::spawn(async move {
            let params = {
                if task.params.is_empty() {
                    String::from("{}")
                } else {
                    task.params
                }
            };
            match task_lua
                .lock()
                .await
                .load(&format!("pcall({}.execute, {})", task.task_name, params))
                .exec()
            {
                Ok(result) => {
                    debug!("{:?}", result);
                }
                Err(e) => {
                    error!("{}", e);
                }
            }
        });

        Ok(())
    }
}

pub struct Scheduler {
    scheduled: HashMap<String, JoinHandle<()>>,
    tasks: Vec<(String, Schedule)>,
}

impl Scheduler {
    pub fn new() -> Result<Scheduler> {
        Ok(Self {
            scheduled: HashMap::new(),
            tasks: vec![],
        })
    }

    pub fn register_task(&mut self, task_name: String, schedule: String) -> Result<()> {
        self.tasks.push((task_name, Schedule::from_str(&schedule)?));
        Ok(())
    }

    pub fn run(&mut self, task_manager: Arc<Mutex<TaskManager>>) -> Result<()> {
        for (task_name, cron) in &self.tasks {
            let mut schedule = false;

            if let Some(task) = self.scheduled.get(task_name) {
                if task.is_finished() {
                    debug!("Task finished: {}", task_name);
                    schedule = true;
                }
            } else {
                schedule = true;
            }

            if schedule {
                let task_manager_cloned = task_manager.clone();
                let duration = (cron.upcoming(Utc).next().unwrap() - Utc::now()).num_milliseconds();
                let task_name = task_name.clone();
                debug!(
                    "Scheduling {} task to run in {} millis",
                    task_name, duration
                );
                self.scheduled.insert(
                    task_name.clone(),
                    tokio::spawn(async move {
                        sleep(Duration::from_millis(duration as u64)).await;
                        let mut task_manager = task_manager_cloned.lock().await;
                        task_manager
                            .schedule(Task {
                                task_name: task_name.clone(),
                                params: String::from(""),
                            })
                            .await
                            .unwrap();
                    }),
                );
            }
        }

        Ok(())
    }
}
