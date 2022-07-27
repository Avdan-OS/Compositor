mod config_loader;

fn main() {
    let config = config_loader::read_config().unwrap();
    println!("Hello, world!");
    println!("{:#?}", config);
}

