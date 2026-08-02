use log::{Level, LevelFilter, Log};

use crate::println;

struct Logger;

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= cfg_select! {
            debug_assertions => Level::Debug,
            _ => Level::Info,
        }
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let file = record.file().unwrap_or("?");
            let loc = alloc::format!(
                "{}:{}",
                file.strip_prefix("crates/cinnamos-kernel").unwrap_or(file),
                record.line().unwrap_or(0),
            );

            println!(
                "[{:>5}] {:<40}: {}",
                record.level(),
                loc,
                record.args(),
            );
        }
    }

    fn flush(&self) {}
}

static LOGGER: Logger = Logger;

pub fn init() {
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(cfg_select! {
        debug_assertions => LevelFilter::Debug,
        _ => LevelFilter::Info,
    });
}
