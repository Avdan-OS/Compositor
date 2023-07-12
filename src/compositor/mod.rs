use slog::{o, Drain, Logger};

use self::backend::{run_udev, run_winit};

mod backend;
mod components;
mod drawing;
mod focus;
mod handlers;
mod input;
mod render;
mod shell;
mod state;

pub fn start() -> Result<(), Box<dyn std::error::Error>> {
    let plain = slog_term::PlainSyncDecorator::new(std::io::stdout());
    let log = Logger::root(slog_term::FullFormat::new(plain).build().fuse(), o!());

    // Env variable NAVDA_BACKEND=udev will tell the compositor to launch under udev,
    // otherwise winit backend.
    if ::std::env::var("NAVDA_BACKEND")
        .map(|s| s == "udev")
        .unwrap_or_default()
    {
        run_udev(log);
    } else {
        run_winit(log);
    }

    Ok(())
}
