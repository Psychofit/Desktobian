//! Logging setup.
//!
//! Verbosity is controlled by the `-v/-q` CLI flags and can be overridden by the
//! standard `RUST_LOG` environment variable for fine-grained, per-module control.

use std::io::Write;

use log::LevelFilter;

/// Initialise the global logger.
///
/// `verbosity` is the net count of `-v` minus `-q` flags; `RUST_LOG`, if set,
/// always wins so power users keep full control.
pub fn init(verbosity: i8) {
    let default = match verbosity {
        i8::MIN..=-2 => LevelFilter::Error,
        -1 => LevelFilter::Warn,
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    let mut builder = env_logger::Builder::new();
    builder.filter_level(default).format(|buf, record| {
        writeln!(
            buf,
            "[{:<5} {}] {}",
            record.level(),
            record.target(),
            record.args()
        )
    });

    if let Ok(env) = std::env::var("RUST_LOG") {
        builder.parse_filters(&env);
    }

    // `try_init` so repeated calls in tests don't panic.
    let _ = builder.try_init();
}
