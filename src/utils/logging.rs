use lazy_static::lazy_static;
use slog::{Logger, Drain, o};

lazy_static! {
    pub static ref LOG: Logger = {
        let plain = slog_term::PlainSyncDecorator::new(std::io::stdout());
        Logger::root(
            slog_term::FullFormat::new(plain)
            .build().fuse(), o!()
        )
    };
}
