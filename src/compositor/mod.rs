//!
//! ## The Wayland Compositor
//! 
//! This module concerns all the nasty
//! Wayland implementation.
//! 
//! Many thanks to Smithay's [anvil](https://github.com/Smithay/smithay/tree/master/anvil
//! and [smallvil](https://github.com/Smithay/smithay/tree/master/smallvil) reference
//! implementations which this is based off.
//! 

mod state;
mod input;
mod handlers;
mod grabs;
mod winit;

use std::error::Error;

use slog::{Logger, Drain};
use smithay::reexports::{wayland_server::Display, calloop::EventLoop};

use self::state::Navda;
pub struct CalloopData {
    state   : Navda,
    display : Display<Navda>
}

pub fn start() -> Result<(), Box<dyn Error>> {
    let log = Logger::root(::slog_stdlog::StdLog.fuse(), slog::o!());
    slog_stdlog::init()?;


    let mut event_loop  : EventLoop<CalloopData>  = EventLoop::try_new()?;

    let mut display     : Display<Navda>    = Display::new()?;
    let state = Navda::new(&mut event_loop, &mut display, log.clone());

    let mut data = CalloopData { state, display };

    winit::init_winit(&mut event_loop, &mut data, log.clone())?;
    
    std::process::Command::new("weston-terminal").spawn().ok();

    event_loop.run(None, &mut data, move |_| {
       // We're running baby! 
       slog::info!(log, "Navda is running!");
    })?;

    Ok(())
}