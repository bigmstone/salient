use std::{io::Write, num::NonZeroU32};

use {
    anyhow::{bail, Context, Result},
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
    serde::{Deserialize, Serialize},
};

use crate::config::Model;

#[derive(Clone, Debug, Serialize, Deserialize)]
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

pub struct AIWorker {
    backend: LlamaBackend,
    model: LlamaModel,
}

impl AIWorker {
    pub fn new(model: &Model) -> Result<Self> {
        let backend = LlamaBackend::init()?;

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

        let model = LlamaModel::load_from_file(&backend, model_path, &model_params)
            .with_context(|| "unable to load model")?;

        Ok(Self { backend, model })
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
        let mut rng = rand::thread_rng();
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(Some(NonZeroU32::new(1024 * 15).unwrap()))
            .with_n_batch(1024 * 15)
            .with_seed(rng.gen());

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
            bail!("Prompt is too long. Cannot have more than {n_len} tokens.")
        }

        std::io::stderr().flush()?;

        let mut batch = LlamaBatch::new(ctx.n_batch() as usize, 1);
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
                    debug!("Hit end of stream");
                    break;
                }

                let output_bytes = self.model.token_to_bytes(new_token_id, Special::Tokenize)?;
                let mut output_string = String::with_capacity(32);
                let _decode_result =
                    decoder.decode_to_string(&output_bytes, &mut output_string, false);
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

    pub fn eval(&mut self, messages: &[Message]) -> Result<Message> {
        debug!("Chat Template: {}", &self.model.get_chat_template(8192)?);

        let prompt = self.build_prompt(messages)?;

        debug!("Prompt: {}", prompt);

        self.llm_run(&prompt)
    }

    fn build_prompt(&self, messages: &[Message]) -> Result<String> {
        let messages: Vec<Value> = messages
            .iter()
            .map(|message| context! { role => message.role, content => message.content })
            .collect();

        let ctx = context! {
            add_generation_prompt => true,
            tools_in_user_message => false,
            // bos_token => "<|begin_of_text|>",
            messages => messages,
        };

        let env = Environment::new();
        Ok(env.render_str(&self.model.get_chat_template(8192)?, ctx)?)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_llm_interface() {
        env_logger::init();
        let messages = [
            Message::new("system", "You are a helpful AI assistant."),
            Message::new("user", "How are you today?"),
        ];

        let model = Model::HuggingFace {
            repo: String::from(""),
            model: String::from(""),
        };

        let mut llm = AIWorker::new(&model).unwrap();
        llm.eval(&messages).unwrap();
    }
}
