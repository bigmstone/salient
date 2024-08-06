pub mod llm;

use std::error::Error;

use llm::Message;

pub struct AIWorker {
    llm: llm::Llm,
}

impl AIWorker {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            llm: llm::Llm::new()?,
        })
    }

    pub fn eval(&mut self, messages: &[Message]) -> Result<Message, Box<dyn Error>> {
        Ok(self.llm.eval(messages)?)
    }
}
