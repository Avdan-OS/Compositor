pub mod core;
use std::error::Error;

use crate::compositor::winit::init_winit;
use crate::core::Config;

pub(crate) use crate::core as Nadva;

mod consts;
pub(crate) use crate::consts as CONST;

mod compositor;

fn main() -> Result<(), Box<dyn Error>> {
    {
        /* let config = config_loader::read_config()
            .unwrap();
        
        println!("{:#?}", config); */

        #![allow(unused_must_use)]
        init_winit();
    }

    let config: Config = Nadva::Config::from_file()?;
    println!("{config:?}");

    Ok(())
}
