use core::fmt;
#[allow(unused)]
use log::{self, Level, LevelFilter, Log, Metadata, Record};
#[allow(unused)]
use log::{debug, error, info, trace, warn};

struct SimpleLogger;

#[allow(unused_parens)]
impl log::Log for SimpleLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if (self.enabled(record.metadata())) {
            print_with_color(
                format_args!("[{}] {}", record.level().to_level_filter(), record.args()),
                log_level_to_color_code(record.level()),
            );
        }
    }

    fn flush(&self) {}
}

pub fn init_log() {
    static LOGGER: SimpleLogger = SimpleLogger;
    log::set_logger(&LOGGER).unwrap();
    log::set_max_level(match option_env!("LOG") {
        Some("error") => LevelFilter::Error,
        Some("warn") => LevelFilter::Warn,
        Some("info") => LevelFilter::Info,
        Some("debug") => LevelFilter::Debug,
        Some("trace") => LevelFilter::Trace,
        _ => LevelFilter::Off,
    });
}

fn log_level_to_color_code(level: Level) -> u8 {
    match level {
        Level::Error => 31,
        Level::Warn => 93,
        Level::Info => 34,
        Level::Debug => 32,
        Level::Trace => 90,
    }
}

#[allow(unused)]
macro_rules! with_color {
    ($args: ident, $color_code: ident) => {{
        let fmt_args = format_args!("\u{1B}[{}m{}\u{1B}[0m", $color_code as u8, $args);
        fmt_args
    }};
}

fn print_with_color(args: fmt::Arguments, color_code: u8) {
    println!(
        "{}",
        format_args!("\u{1B}[{}m{}\u{1B}[0m", color_code as u8, args)
    );
}
