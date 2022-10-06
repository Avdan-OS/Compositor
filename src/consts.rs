use std::path::Path;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref CONFIG_FOLDER : &'static Path = Path::new("/etc/avdan");
    pub static ref CONFIG_FILE   : &'static Path = Path::new("compositor.jsonc");
}
