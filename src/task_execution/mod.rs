use std::{
    any::{Any, TypeId},
    collections::HashMap,
    error::Error,
    sync::{Arc, Mutex as StdMutex},
};

use {
    mlua::{prelude::*, LuaSerdeExt},
    serde_json::Value as JsonValue,
    tokio::sync::{mpsc, Mutex},
};

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
    tx: mpsc::Sender<Task>,
}

impl TaskManager {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let (tx, mut rx): (mpsc::Sender<Task>, mpsc::Receiver<Task>) = mpsc::channel(100);

        let lua = Arc::new(Mutex::new(unsafe { Lua::unsafe_new() }));

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

        let lua_locked = lua.lock().await;

        let mut scripts = [(
            vec![String::from("Eval")],
            include_str!("./scripts/script.lua"),
        )];

        let mut tasks = vec![];

        for (local_tasks, script) in scripts.iter_mut() {
            lua_locked.load(script.to_owned()).exec()?;

            for task in local_tasks {
                lua_locked.load(&format!("{}.setup()", task)).exec()?;
                tasks.push(task.clone());
            }
        }

        drop(lua_locked);

        Ok(Self {
            lua,
            tasks,
            tx,
            scope: Arc::new(StdMutex::new(Scope::new())),
        })
    }

    pub async fn register_function<F>(
        &mut self,
        name: &str,
        function: F,
    ) -> Result<(), Box<dyn Error>>
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
        self.tx.send(task).await?;
        Ok(())
    }
}
