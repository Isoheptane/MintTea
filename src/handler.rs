use async_trait::async_trait;
use frankenstein::client_reqwest::Bot;

pub type HandlerResult = anyhow::Result<std::ops::ControlFlow<(), ()>>;

#[async_trait]
pub trait UpdateHandler<D, U>: Send + Sync where {
    async fn handle(&self, bot: Bot, data: &D, update: &U) -> HandlerResult;
}

pub struct HandlerChain<D, U> where {
    chain: Vec<Box<dyn UpdateHandler<D, U>>>
}

impl<D, U> HandlerChain<D, U> where {
    pub fn new() -> HandlerChain<D, U> {
        HandlerChain { chain: vec![] }
    }

    pub fn add_handler(&mut self, handler: impl UpdateHandler<D, U> + 'static) {
        self.chain.push(Box::new(handler));
    }

    pub async fn run_chain(&self, bot: Bot, data: &D, update: &U) -> HandlerResult {
        for handler in self.chain.iter() {
            let result = handler.handle(bot.clone(), data, update).await?;
            match result {
                std::ops::ControlFlow::Continue(_) => { continue; }
                std::ops::ControlFlow::Break(_) => { return Ok(result); }
            }
        }
        Ok(std::ops::ControlFlow::Continue(()))
    }
}

impl<D, U> From<Vec<Box<dyn UpdateHandler<D, U>>>> for HandlerChain<D, U> where D: Clone {
    fn from(value: Vec<Box<dyn UpdateHandler<D, U>>>) -> Self {
        HandlerChain { chain: value }
    }
}