use clap::crate_name;
use colorful::{core::color_string::CString, Colorful};
pub use log::{debug, error, info, trace, warn, Level};

pub struct SimpleLogger {
    max_level: Level,
}

impl SimpleLogger {
    pub fn new() -> Self {
        Self {
            max_level: Level::Info,
        }
    }

    pub fn with_level(mut self, max_level: Level) -> Self {
        self.max_level = max_level;
        self
    }

    pub fn init(self) -> Result<(), ::log::SetLoggerError> {
        ::log::set_max_level(::log::LevelFilter::Debug);
        ::log::set_boxed_logger(Box::new(self))
    }
}

impl ::log::Log for SimpleLogger {
    fn enabled(&self, metadata: &::log::Metadata) -> bool {
        metadata.level() <= self.max_level
            && metadata.target().split(':').next().unwrap() == crate_name!()
    }

    fn log(&self, record: &::log::Record) {
        if self.enabled(record.metadata()) {
            let prefix: CString;
            match record.level() {
                Level::Debug => prefix = "debug".blue().bold(),
                Level::Error => prefix = "error".red().bold(),
                Level::Info => prefix = "info".light_blue().bold(),
                Level::Trace => prefix = "info".magenta().bold(),
                Level::Warn => prefix = "warn".yellow().bold(),
            }
            println!("{}: {}", prefix, record.args());
        }
    }

    fn flush(&self) {}
}
