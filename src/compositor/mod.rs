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
mod backends;
mod components;

use std::error::Error;

use slog::{Logger, Drain};
use smithay::reexports::{wayland_server::Display, calloop::EventLoop};

use self::{state::Navda, backends::NavdaBackend};
pub struct CalloopData<BEnd : 'static> {
    state   : Navda<BEnd>,
    display : Display<Navda<BEnd>>
}

pub fn start() -> Result<(), Box<dyn Error>> {
    let log = Logger::root(::slog_stdlog::StdLog.fuse(), slog::o!());
    slog_stdlog::init()?;

    backends::Winit::run(log)?;

    Ok(())
}