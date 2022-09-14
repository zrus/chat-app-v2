use chrono::{Datelike, Utc};
use log::{LevelFilter, SetLoggerError};
use log4rs::{
  append::{
    console::ConsoleAppender,
    rolling_file::{
      policy::compound::{
        roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger, CompoundPolicy,
      },
      RollingFileAppender,
    },
  },
  config::{Appender, Root},
  encode::pattern::PatternEncoder,
  filter::threshold::ThresholdFilter,
  Config, Handle,
};

use crate::constants::{LOG_DEBUG_PATTERN, LOG_DIR, LOG_PATTERN};

pub fn setup_logger(
  level: LevelFilter,
  file_log: Option<&FileLoggerSetting>,
) -> Result<Handle, SetLoggerError> {
  let config_builder = Config::builder();
  let root_builder = Root::builder();

  let (config_builder, root_builder) = match file_log {
    Some(file_log) => {
      let fixed_window_roller = FixedWindowRoller::builder()
        .build(
          &format!("{}.{{}}.log", file_log.name()),
          file_log.rotation(),
        )
        .unwrap();
      let size_trigger = SizeTrigger::new(file_log.size());
      let compound_policy =
        CompoundPolicy::new(Box::new(size_trigger), Box::new(fixed_window_roller));

      let pattern = match file_log.level() {
        LevelFilter::Debug => LOG_DEBUG_PATTERN,
        _ => LOG_PATTERN,
      };

      (
        config_builder.appender(
          Appender::builder()
            .filter(Box::new(ThresholdFilter::new(*file_log.level())))
            .build(
              "logfile",
              Box::new(
                RollingFileAppender::builder()
                  .encoder(Box::new(PatternEncoder::new(pattern))) //TODO: add pattern
                  .build(
                    &format!("{}/{}.log", LOG_DIR, file_log.name()),
                    Box::new(compound_policy),
                  )
                  .unwrap(),
              ),
            ),
        ),
        root_builder.appender("logfile"),
      )
    }
    None => (config_builder, root_builder),
  };

  let pattern = match level {
    LevelFilter::Debug => LOG_DEBUG_PATTERN,
    _ => LOG_PATTERN,
  };

  let config = config_builder
    .appender(
      Appender::builder()
        .filter(Box::new(ThresholdFilter::new(level)))
        .build(
          "console",
          Box::new(
            ConsoleAppender::builder()
              .encoder(Box::new(PatternEncoder::new(pattern)))
              .build(),
          ),
        ),
    )
    .build(root_builder.appender("console").build(LevelFilter::Trace))
    .unwrap();

  Ok(log4rs::init_config(config)?)
}

pub struct FileLoggerSettingBuilder {
  level: LevelFilter,
  name: String,
  size: u64,
  rotation: u32,
}

impl Default for FileLoggerSettingBuilder {
  fn default() -> Self {
    let now = Utc::now();
    Self {
      name: format!("{}-{}-{}", now.day(), now.month(), now.year()),
      size: 10 * 1024 * 1024,
      rotation: 10,
      level: LevelFilter::Debug,
    }
  }
}

impl FileLoggerSettingBuilder {
  pub fn level(mut self, level: LevelFilter) -> Self {
    self.level = level;
    self
  }

  pub fn name(mut self, name: Option<&str>) -> Self {
    match name {
      Some(name) => self.name = name.to_owned(),
      None => {}
    }
    self
  }

  pub fn size(mut self, size: Option<u64>) -> Self {
    match size {
      Some(size) => self.size = size,
      None => {}
    }
    self
  }

  pub fn rotation(mut self, rotation: Option<u32>) -> Self {
    match rotation {
      Some(rotation) => self.rotation = rotation,
      None => {}
    }
    self
  }

  pub fn build(self) -> FileLoggerSetting {
    FileLoggerSetting {
      level: self.level,
      name: self.name,
      size: self.size,
      rotation: self.rotation,
    }
  }
}

#[derive(Debug)]
pub struct FileLoggerSetting {
  level: LevelFilter,
  name: String,
  size: u64,
  rotation: u32,
}

impl Default for FileLoggerSetting {
  fn default() -> Self {
    FileLoggerSettingBuilder::default().build()
  }
}

impl FileLoggerSetting {
  pub fn level(&self) -> &LevelFilter {
    &self.level
  }

  pub fn name(&self) -> &str {
    &self.name
  }

  pub fn size(&self) -> u64 {
    self.size
  }

  pub fn rotation(&self) -> u32 {
    self.rotation
  }
}
