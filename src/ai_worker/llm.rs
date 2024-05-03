use std::{io::Write, num::NonZeroU32, path::PathBuf, time::SystemTimeError};

use {
    anyhow::{bail, Context, Result},
    chrono::Local,
    hf_hub::api::sync::ApiBuilder,
    llama_cpp_2::{
        context::params::LlamaContextParams,
        llama_backend::LlamaBackend,
        llama_batch::LlamaBatch,
        model::{
            params::LlamaModelParams,
            LlamaModel, {AddBos, Special},
        },
        token::data_array::LlamaTokenDataArray,
    },
    log::debug,
    minijinja::{context, Environment, Value},
    rand::prelude::*,
    serde_json::{json, Value as JsonValue},
};

const FUNCTION_TEMPLATE: &str = r#"You are a function calling AI model. You are provided with function signatures within <tools></tools> XML tags. You may call one or more functions to assist with the user query. Don't make assumptions about what values to plug into functions, use exactly the parameters provided you here using the below pydynamic schema. Ensuring that you format your output as valid JSON. Here are the available tools:
<tools>
{% for function in functions %}{'type': 'function', 'function': {'name': '{{function['name']}}', 'description': '{{function['description']}}', 'parameters': {{function['parameters']|tojson}} }}
{% endfor %}
</tools>
Use the following pydantic model json schema for each tool call you will make:
{'title': 'FunctionCall', 'type': 'object', 'properties': {'arguments': {'title': 'Arguments', 'type': 'object'}, 'name': {'title': 'Name', 'type': 'string'}}, 'required': ['arguments', 'name']}"#;

pub enum Model {
    /// Use an already downloaded model
    Local {
        /// The path to the model. e.g. `/home/marcus/.cache/huggingface/hub/models--TheBloke--Llama-2-7B-Chat-GGUF/blobs/08a5566d61d7cb6b420c3e4387a39e0078e1f2fe5f055f3a03887385304d4bfa`
        path: PathBuf,
    },
    /// Download a model from huggingface (or use a cached version)
    HuggingFace {
        /// the repo containing the model. e.g. `TheBloke/Llama-2-7B-Chat-GGUF`
        repo: String,
        /// the model name. e.g. `llama-2-7b-chat.Q4_K_M.gguf`
        model: String,
    },
}

impl Model {
    /// Convert the model to a path - may download from huggingface
    fn get_or_load(self) -> Result<PathBuf> {
        match self {
            Model::Local { path } => Ok(path),
            Model::HuggingFace { model, repo } => ApiBuilder::new()
                .with_progress(true)
                .build()
                .with_context(|| "unable to create huggingface api")?
                .model(repo)
                .get(&model)
                .with_context(|| "unable to download model"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Message {
    role: String,
    content: String,
}

impl Message {
    pub fn new(role: &str, content: &str) -> Self {
        Self {
            role: String::from(role),
            content: String::from(content),
        }
    }
}

pub struct Llm {
    backend: LlamaBackend,
    model: LlamaModel,
    rng: ThreadRng,
    tools: Value,
}

impl Llm {
    pub fn new() -> Result<Self> {
        let backend = LlamaBackend::init()?;

        let model = Model::HuggingFace {
            repo: String::from("QuantFactory/dolphin-2.9-llama3-8b-GGUF"),
            model: String::from("dolphin-2.9-llama3-8b.Q8_0.gguf"),
        };

        let model_path = model
            .get_or_load()
            .with_context(|| "failed to get model from args")?;

        let tools = context! {
            functions => vec![
                context! {
                    name => "noop",
                    description => "Execute this function when the context from the user does not require a specific function to be run.",
                    parameters => ""
                },
                context! {
                    name => "get_weather",
                    description => "Get the weather for a specific location.",
                    parameters => context! {
                        location => context! {
                            type => "string",
                            description => "String of the location in City, State format you want to get the weather information for."
                        }
                    }
                },
            ],
        };

        let model_params = {
            #[cfg(feature = "cublas")]
            if !disable_gpu {
                LlamaModelParams::default().with_n_gpu_layers(1000)
            } else {
                LlamaModelParams::default()
            }
            #[cfg(not(feature = "cublas"))]
            LlamaModelParams::default()
        };

        let model = LlamaModel::load_from_file(&backend, model_path, &model_params)
            .with_context(|| "unable to load model")?;

        Ok(Self {
            backend,
            model,
            rng: rand::thread_rng(),
            tools,
        })
    }

    pub fn _load_model(&mut self, model: Model) -> Result<()> {
        let model_path = model
            .get_or_load()
            .with_context(|| "failed to get model from args")?;

        let model_params = {
            #[cfg(feature = "cublas")]
            if !disable_gpu {
                LlamaModelParams::default().with_n_gpu_layers(1000)
            } else {
                LlamaModelParams::default()
            }
            #[cfg(not(feature = "cublas"))]
            LlamaModelParams::default()
        };

        self.model = LlamaModel::load_from_file(&self.backend, model_path, &model_params)
            .with_context(|| "unable to load model")?;

        Ok(())
    }

    fn llm_run(&mut self, prompt: &str) -> Result<Message> {
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(Some(NonZeroU32::new(1024 * 4).unwrap()))
            .with_seed(self.rng.gen());

        let mut ctx = self
            .model
            .new_context(&self.backend, ctx_params)
            .with_context(|| "unable to create the llama_context")?;

        let tokens_list = self
            .model
            .str_to_token(prompt, AddBos::Always)
            .with_context(|| format!("failed to tokenize {prompt}"))?;

        let n_cxt = ctx.n_ctx() as i32;
        let n_len = prompt.len() as i32;
        let n_kv_req = tokens_list.len() as i32 + (n_len - tokens_list.len() as i32);

        debug!("n_len = {n_len}, n_ctx = {n_cxt}, k_kv_req = {n_kv_req}");

        if n_kv_req > n_cxt {
            bail!(
                "n_kv_req > n_ctx, the required kv cache size is not big enough either reduce n_len or increase n_ctx"
            )
        }

        if tokens_list.len() >= usize::try_from(n_len)? {
            bail!("the prompt is too long, it has more tokens than n_len")
        }

        for token in &tokens_list {
            eprint!("{}", self.model.token_to_str(*token, Special::Tokenize)?);
        }

        std::io::stderr().flush()?;

        let mut batch = LlamaBatch::new(1024, 1);

        let last_index: i32 = (tokens_list.len() - 1) as i32;
        for (i, token) in (0_i32..).zip(tokens_list.into_iter()) {
            let is_last = i == last_index;
            batch.add(token, i, &[0], is_last)?;
        }

        ctx.decode(&mut batch)
            .with_context(|| "llama_decode() failed")?;

        let mut n_cur = batch.n_tokens();
        let mut decoder = encoding_rs::UTF_8.new_decoder();
        let mut result = vec![];
        while n_cur <= n_len {
            {
                let candidates = ctx.candidates_ith(batch.n_tokens() - 1);
                let mut candidates_p = LlamaTokenDataArray::from_iter(candidates, false);
                let new_token_id = candidates_p.sample_token(&mut ctx);

                if new_token_id == self.model.token_eos() {
                    println!();
                    debug!("Hit end of stream");
                    break;
                }

                let output_bytes = self.model.token_to_bytes(new_token_id, Special::Tokenize)?;
                let mut output_string = String::with_capacity(32);
                let _decode_result =
                    decoder.decode_to_string(&output_bytes, &mut output_string, false);
                print!("{}", output_string);
                std::io::stdout().flush()?;
                result.push(output_string);

                batch.clear();
                batch.add(new_token_id, n_cur, &[0], true)?;
            }

            n_cur += 1;

            ctx.decode(&mut batch).with_context(|| "failed to eval")?;
        }
        debug!("{}", result.join(""));

        Ok(Message::new("assistant", &result.join("")))
    }

    fn eval_function(&mut self, messages: &[Message]) -> Result<Message> {
        let mut messages = Vec::from(messages);
        let env = Environment::new();
        let function_prompt = env.render_str(FUNCTION_TEMPLATE, &self.tools)?;

        messages.push(Message::new("system", &function_prompt));

        for _ in 0..10 {
            let prompt = format!("{}\nFunction call:\n", self.build_prompt(&messages)?);
            debug!("Prompt: {}", prompt);
            let out = self.llm_run(&prompt)?;

            debug!("Output: {:#?}", out);

            match serde_json::from_str::<JsonValue>(out.content.trim()) {
                Ok(_json) => return Ok(Message::new("assistant", out.content.trim())),
                Err(err) => {
                    debug!("Error in function result: {}", err);
                    messages.push(Message::new(
                        "system",
                        &format!("Error Parsing JSON: {}", err),
                    ));
                }
            }
        }

        bail!("Too many retries");
    }

    pub fn eval(&mut self, messages: &[Message]) -> Result<Message> {
        debug!("{}", &self.model.get_chat_template(2048).unwrap());

        let func_output = self.eval_function(messages)?;

        // let env = Environment::new();
        // let function_prompt = env.render_str(FUNCTION_TEMPLATE, &self.tools)?;

        // let ctx = context! {
        //     bos_token => "<|begin_of_text|>",
        //     messages => vec![
        //         context! {
        //             role => "system",
        //             content => format!("{}\n\nFunction call:", function_prompt),
        //         },
        //         context! {
        //             role => "user",
        //             content => "What's the weather for 71270?",
        //         },
        //     ],
        // };

        // let prompt = env.render_str(&self.model.get_chat_template(2048)?, ctx)?;

        // debug!("Prompt: {}", prompt);

        // self.llm_run(&prompt)
        Ok(func_output)
    }

    fn build_prompt(&self, messages: &[Message]) -> Result<String> {
        let messages: Vec<Value> = messages
            .iter()
            .map(|message| context! { role => message.role, content => message.content })
            .collect();

        let ctx = context! {
            bos_token => "<|begin_of_text|>",
            messages => messages,
        };

        let env = Environment::new();
        Ok(env.render_str(&self.model.get_chat_template(2048)?, ctx)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_function_call_weather() {
        let mut llm = Llm::new().unwrap();
        let out = llm.eval(&[
            Message::new("system", "You are Leroy, a helpful AI assistant."),
            Message::new(
                "system",
                "We need to get the latest weather for Matthew. He is curently in Ruston, Louisiana.",
            )]).unwrap();

        let json = serde_json::from_str::<JsonValue>(&out.content).unwrap();

        assert_eq!(json["name"], "get_weather");
        assert_eq!(json["arguments"]["location"], "Ruston, Louisiana");
    }

    #[test]
    fn test_function_call_noop() {
        let mut llm = Llm::new().unwrap();
        let out = llm
            .eval(&[
                Message::new("system", "You are Leroy, a helpful AI assistant."),
                Message::new("user", "I'd like to discuss philosphy with you."),
            ])
            .unwrap();

        let json = serde_json::from_str::<JsonValue>(&out.content).unwrap();

        assert_eq!(json["name"], "noop");
    }
}
