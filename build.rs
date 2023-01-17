use std::{
    fs,
    path::{
        Path,
        PathBuf,
    },

    env,
};
const CONFIG_FILE: &'static str   = "compositor.json";

fn main() -> std::io::Result<()> {
    let config_folder = if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        Path::new(&xdg_config)
            .to_path_buf()
    } else {
        let home = std::env::var("HOME").expect("Waaa! $HOME not set? Not my problem.");
        Path::new(&home)
            .join(".config")
    };

    let config_path: PathBuf = config_folder    
            .join(CONFIG_FILE);

    fs::create_dir_all(config_folder)?;
    
    overwrite_if_set(&config_path)
}

///
/// Overwrites the config file unless
/// OVERWRITE is manually set to `false`, or any non-`true` value.
/// 
fn overwrite_if_set(path: &Path) -> Result<(), std::io::Error> {
    #![allow(clippy::or_fun_call)]
    let overwrite: String = env::var("OVERWRITE")
        .unwrap_or("true".into());
    
    if Path::exists(path) &&
            overwrite.to_lowercase().eq("true") {
        fs::remove_file(path)?;
        fs::copy(CONFIG_FILE, path)?;
    }

    Ok(())
}
