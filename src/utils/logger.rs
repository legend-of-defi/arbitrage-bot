use chrono::Local;
use eyre::{Error, Result};
use fern::Dispatch;

/// Sets up the application logger with file and console output.
///
/// # Returns
/// * `Result<(), Error>` - Success or failure of logger setup
///
/// # Errors
/// * If log file creation fails
/// * If logger configuration fails
pub fn setup_logger() -> Result<(), Error> {
    Dispatch::new()
        // Set the default logging level
        .level(log::LevelFilter::Info)
        // Configure logging to console
        .chain(std::io::stdout())
        // Format log messages with time and log level
        .format(|out, message, record| {
            out.finish(format_args!(
                "{} [{}] {}",
                Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                message
            ));
        })
        .apply()?;
    Ok(())
}
