use std::sync::atomic::Ordering;

use smithay::{
    reexports::wayland_server::protocol::wl_surface::WlSurface, 
    output::Output
};

use super::state::Navda;

mod winit;
// pub mod udev ;

pub use winit::Winit;

pub trait Backend {
    fn seat_name(&self) -> String;
    fn reset_buffers(&mut self, output: &Output);
    fn early_import(&mut self, surface: &WlSurface);
}

pub trait NavdaBackend {
    type Data : 'static;
    fn run(logger : slog::Logger) -> Result<(), Box<dyn std::error::Error>>;

    fn stop_compositor(state : &mut Navda<Self::Data>) {
        state.running.store(false, Ordering::SeqCst);
    }
}

