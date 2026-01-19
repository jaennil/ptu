use std::fs::File;

use color_eyre::eyre;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub(crate) fn init() -> eyre::Result<()> {
    let log_file = File::create("/tmp/ptu.log")?;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_writer(log_file)
                .with_ansi(false)
                .with_target(true),
        )
        .init();

    tracing::info!("logging initialized");

    Ok(())
}
