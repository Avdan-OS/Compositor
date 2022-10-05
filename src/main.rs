#![allow(irrefutable_let_patterns)]

mod consts;
mod compositor;
pub mod core;

use crate::consts as CONST;
pub(crate) use crate::config::Config;
pub(crate) use crate::core as Nadva;

use smithay::reexports::{
    calloop::EventLoop,
    wayland_server::Display
};

use slog::{
    Drain,
    Logger,
};

pub use compositor::{
    state::AvCompositor,
    init::init as initialize
};

use std::error::Error;

use std::process::Command;

pub struct CalloopData {
    state  : AvCompositor,
    display: Display<AvCompositor>,
}

pub mod config;

fn main() -> Result<(), Box<dyn Error>> {
    println!();
    println!();
    // Load Nadva's Config
    Config::load().unwrap();

    println!("window {:?}", &Config::config().keybinds.window);
    println!();
    println!();

    initialize()?;
    
    Ok(())
}
