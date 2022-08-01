use std::{fs, path::Path, env,};

const CONFIG_FOLDER: &'static str = "/etc/AvdanOS";
const CONFIG_FILE: &'static str = "Compositor.json";

fn main() -> std::io::Result<()> {
    let config_path = Path::new(CONFIG_FOLDER)
        .join(CONFIG_FILE);

    fs::create_dir_all(CONFIG_FOLDER)?;
    
    overwrite_if_set(&config_path)
}

///
/// Overwrites the config file unless
/// OVERWRITE is manually set to `false`, or any non-`true` value.
/// 
fn overwrite_if_set(path: &Path) -> Result<(), std::io::Error> {
    let overwrite = env::var("OVERWRITE")
        .unwrap_or("true".into());
    

    if  Path::exists(path) &&
        overwrite.to_lowercase().eq("true")
    {
        fs::remove_file(path)?;
        fs::copy(CONFIG_FILE, path)?;
    }
    Ok(())
}