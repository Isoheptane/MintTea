pub type HandlerResult = anyhow::Result<std::ops::ControlFlow<(), ()>>;
pub type ModalHandlerResult = anyhow::Result<()>;