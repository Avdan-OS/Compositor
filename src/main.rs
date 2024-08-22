use slog::info;

use crate::utils::LOG;
mod utils;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    info!(LOG, "Logging ready!");

    Ok(())
}
