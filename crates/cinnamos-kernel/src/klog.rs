use core::fmt::Write;

use log::{LevelFilter, Log};

use crate::{console::Console, hloc};

struct Location<'a> {
    file: &'a str,
    line: u32,
}

impl core::fmt::Display for Location<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let line_len = {
            let mut i = 0usize;
            let mut line = self.line;
            while line > 0 {
                line /= 10;
                i += 1;
            }
            i
        };
        let file = self
            .file
            .strip_prefix("crates/cinnamos-kernel/")
            .unwrap_or(self.file);
        let file_len = file.len();
        let len = file_len + 1 + line_len;

        if let Some(width) = f.width()
            && let Some(pad) = width.checked_sub(len)
            && let Some(align) = f.align()
        {
            let pad_left = match align {
                core::fmt::Alignment::Right => pad,
                core::fmt::Alignment::Center => pad / 2,
                _ => 0,
            };
            for _ in 0..pad_left {
                write!(f, " ")?;
            }
        }
        write!(f, "{}:{}", file, self.line)?;
        if let Some(width) = f.width()
            && let Some(pad) = width.checked_sub(len)
            && let Some(align) = f.align()
        {
            let pad_right = match align {
                core::fmt::Alignment::Left => pad,
                core::fmt::Alignment::Center => pad.div_ceil(2),
                _ => 0,
            };
            for _ in 0..pad_right {
                write!(f, " ")?;
            }
        }
        Ok(())
    }
}

struct Logger;

impl Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &log::Record) {
        if self.enabled(record.metadata()) {
            let loc: Location<'_> = Location {
                file: record.file().unwrap_or("?"),
                line: record.line().unwrap_or(0),
            };
            let hid = hloc::hart_local().hid;

            let mut writer = Console::lock();
            let _ = writeln!(
                writer,
                "[{:>2}][{:>5}] {:<32}: {}",
                hid,
                record.level(),
                loc,
                record.args()
            );
        }
    }

    fn flush(&self) {
        Console::lock().flush();
    }
}

static LOGGER: Logger = Logger;

pub fn init() {
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(cfg_select! {
        debug_assertions => LevelFilter::Trace,
        _ => LevelFilter::Info,
    });
}
