use std::fs;

fn main() {
    fs::copy("Compositor.json", "/etc/AvdanOS/Compositor.json")
        .expect("Copy failed to execute.");
}
