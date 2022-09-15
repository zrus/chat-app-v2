use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait TPeer: Send {
  async fn run(&mut self, boot_nodes: &[&str]) -> Result<()>;
}

#[async_trait]
pub trait TBuilder {
  fn boxed(self) -> Box<dyn TBuilder>;
  async fn build(&self) -> Result<Box<dyn TPeer>>;
}
