use slog::info;

use crate::utils::LOG;
mod utils;

#[tokio::main]
async fn main() {
    info!(LOG, "Logging ready!");
}
