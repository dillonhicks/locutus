use crate::deps::tracing::Level;

pub fn try_initialize(log_level: Option<Level>) -> Result<(), Box<dyn std::error::Error>> {
    let log_level = log_level.unwrap_or(Level::INFO);

    let subscriber = crate::deps::tracing_subscriber::fmt()
        .with_max_level(log_level)
        // completes the builder
        .finish();

    crate::deps::tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}
