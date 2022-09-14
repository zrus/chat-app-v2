use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait TPeer {
  async fn run(&mut self) -> Result<()>;
}

#[async_trait]
pub trait TBuilder {
  async fn build(&self) -> Result<Box<dyn TPeer>>;
}
