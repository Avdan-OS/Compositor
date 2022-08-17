pub mod core;
use std::error::Error;

use crate::core::Config;

pub(crate) use crate::core as Nadva;

mod consts;
pub(crate) use crate::consts as CONST;

mod wayland;
pub(crate) use crate::wayland::display as Display;

fn main() -> Result<(), Box<dyn Error>> {
    {
        Display::display_server();
        
        /* let config = config_loader::read_config()
            .unwrap();
        
        println!("{:#?}", config); */
    }

    let config: Config = Nadva::Config::from_file()?;
    println!("{config:?}");

    Ok(())
}
