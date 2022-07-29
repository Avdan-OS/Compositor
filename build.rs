use std::fs;

fn main() -> std::io::Result<()> {
    fs::create_dir("/etc/AvdanOS")?;
    fs::copy("Compositor.json", "/etc/AvdanOS/Compositor.json")?;
    Ok(())
}
