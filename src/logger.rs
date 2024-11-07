use std::str::FromStr;

use anyhow::Result;
use tracing::Level;
use tracing_error::ErrorLayer;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init(log_level: &str) -> Result<()> {
    let log_level = Level::from_str(log_level)?;
    let env_filter = EnvFilter::builder().with_default_directive(log_level.into());
    let env_filter = env_filter
        .try_from_env()
        .or_else(|_| env_filter.with_env_var("info").from_env())?;
    let console_layer = fmt::layer()
        .with_ansi(true)
        .with_line_number(true)
        .with_level(true)
        .with_filter(env_filter);
    tracing_subscriber::registry()
        .with(console_layer)
        .with(ErrorLayer::default())
        .try_init()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_log() {
        use super::*;
        init("info").unwrap();
        tracing::info!("hello world");
    }
}
