use slog::{Drain, Duplicate, Logger};
use sloggers::file::FileLoggerBuilder;
use sloggers::types::Severity;
use sloggers::Build;
use std::path::Path;

/// This returns a logger that also logs to the file pointed by the path parameter on top of the
/// provided logger. The returned logger logs to both outputs.
///
/// The file logger has the debug level since this is what we want for debugging.
pub fn file_logger(logger: Logger, path: &Path) -> Logger {
    let mut dest = path.to_path_buf();
    dest.push("logs.txt");
    let file_drain = FileLoggerBuilder::new(dest)
        .level(Severity::Debug)
        .build()
        .expect("Could not create a file logger");
    let drain = Duplicate::new(logger, file_drain).fuse();
    Logger::root(drain, o!())
}
