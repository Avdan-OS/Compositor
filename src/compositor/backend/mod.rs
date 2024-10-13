mod udev;
mod winit;

pub use udev::{run_udev, UdevData};
pub use winit::run_winit;

use smithay::{output::Output, reexports::wayland_server::protocol::wl_surface::WlSurface};

pub trait Backend {
    const HAS_RELATIVE_MOTION: bool = false;
    fn seat_name(&self) -> String;
    fn reset_buffers(&mut self, output: &Output);
    fn early_import(&mut self, surface: &WlSurface);
}
