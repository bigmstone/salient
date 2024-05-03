pub mod llm;

use std::error::Error;

use {
    log::debug,
    serde::{Deserialize, Serialize},
};

use llm::Message;

// // { "model": "llava-1.6-vicuna", "messages": [{"role": "user", "content": [{"type":"text", "text": "What is in the image?"}, {"type": "image_url", "image_url": {"url": "https://upload.wikimedia.org/wikipedia/commons/thumb/d/dd/Gfp-wisconsin-madison-the-nature-boardwalk.jpg/2560px-Gfp-wisconsin-madison-the-nature-boardwalk.jpg" }}], "temperature": 0.9}]}
// #[derive(Serialize, Deserialize)]
// pub enum Models {
//     #[serde(rename = "llava-1.6-vicuna")]
//     Vicuna,
//     #[serde(rename = "gpt-4")]
//     GPT4,
//     #[serde(rename = "llama3-8b-instruct")]
//     Llama3,
// }

// #[derive(Serialize, Deserialize)]
// pub enum Roles {
//     #[serde(rename = "user")]
//     User,
//     #[serde(rename = "system")]
//     System,
//     #[serde(rename = "assistant")]
//     Assistant,
// }

// #[derive(Serialize, Deserialize)]
// pub struct Choice {
//     pub finish_reason: String,
//     pub index: i32,
//     pub logprobs: Option<i32>,
//     pub message: Message,
// }

// #[derive(Serialize, Deserialize)]
// pub struct Content {
//     #[serde(rename = "type")]
//     pub content_type: String,
//     pub text: String,
// }

// #[derive(Serialize, Deserialize)]
// pub struct Message {
//     pub role: Roles,
//     pub content: String,
//     pub function_call: Option<i32>,
//     pub tool_calls: Option<i32>,
// }

// #[derive(Serialize, Deserialize)]
// pub struct ChatCompletion {
//     pub model: Models,
//     pub messages: Vec<Message>,
//     pub temperature: f32,
// }

// #[derive(Serialize, Deserialize)]
// pub struct Response {
//     pub created: u32,
//     pub object: String,
//     pub id: String,
//     pub model: String,
//     pub choices: Vec<Choice>,
//     pub usage: Usage,
// }

// #[derive(Serialize, Deserialize)]
// pub struct Usage {
//     prompt_tokens: u32,
//     completion_tokens: u32,
//     total_tokens: u32,
// }

pub struct AIWorker {
    llm: llm::Llm,
}

impl AIWorker {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            llm: llm::Llm::new()?,
        })
    }

    pub async fn eval(&mut self, messages: &[Message]) -> Result<(), Box<dyn Error>> {
        self.llm.eval(messages)?;
        Ok(())
    }
}
