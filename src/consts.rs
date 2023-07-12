use std::path::{Path, PathBuf};

use lazy_static::lazy_static;

use crate::compat::{BaseDir, XdgBaseDir};

lazy_static! {
    pub static ref CONFIG_FOLDER: PathBuf = { XdgBaseDir::Config.path().join("avdan") };
    pub static ref CONFIG_FILE: &'static Path = Path::new("Compositor.jsonc");
}
