use chrono::Local;
use eyre::Result;
use fern::Dispatch;

/// Sets up the application logger with file and console output.
///
/// # Returns
/// * `Result<()>` - Success or failure of logger setup
///
/// # Errors
/// * If log file creation fails
/// * If logger configuration fails
pub fn setup_logger() -> Result<()> {
    Dispatch::new()
        // Set logging level from RUST_LOG env var or default to Info
        .level(
            std::env::var("RUST_LOG")
                .map(|level| level.parse().unwrap_or(log::LevelFilter::Info))
                .unwrap_or(log::LevelFilter::Info),
        )
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
