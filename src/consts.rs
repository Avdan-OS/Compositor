use std::path::{Path, PathBuf};

use compositor_macros::navda_config;
use lazy_static::lazy_static;

lazy_static! {
    pub static ref CONFIG_FOLDER : PathBuf = {
        navda_config!()
    };
    pub static ref CONFIG_FILE   : &'static Path = Path::new("Compositor.jsonc");
}
