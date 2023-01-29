use slog::{Logger, Drain, o};

use self::backend::run_udev;

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
    let log = Logger::root(
        slog_term::FullFormat::new(plain)
        .build().fuse(), o!()
    );
    run_udev(log);
    Ok(())
}
