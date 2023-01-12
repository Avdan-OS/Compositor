//!
//! # Navda
//! 
//! The Wayland compositor behind AvdanOS.
//! 
//! Based off the [Smithay](https://github.com/Smithay/smithay)
//! library. 
//!  
//! If you are looking for user-oriented documentation,
//! you are in the wrong place! Please use
//! [docs.avdanos.org](https://docs.avdanos.org) instead. 
//! 

mod consts;
pub mod core;
mod compositor;

use crate::consts as CONST;
pub(crate) use crate::config::Config;

use std::error::Error;

pub mod config;

fn main() -> Result<(), Box<dyn Error>> {
    println!("\n");
    // Load Nadva's Config
    Config::load().unwrap();
    

    compositor::start()?;
    Ok(())
}
