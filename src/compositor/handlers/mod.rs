mod compositor;
mod xdg_shell;

use smithay::{
    input::{
        SeatHandler, Seat,
        pointer::CursorImageStatus
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    delegate_seat, wayland::data_device::{DataDeviceHandler, DataDeviceState, ClientDndGrabHandler, ServerDndGrabHandler}, delegate_data_device, delegate_output
};

use super::state::Navda;

impl SeatHandler for Navda {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;

    fn seat_state(&mut self) -> &mut smithay::input::SeatState<Self> {
        &mut self.seat_state
    }

    fn cursor_image(
        &mut self,
        _seat   : &Seat<Self>,
        _image  : CursorImageStatus,
    ) {}

    fn focus_changed(
        &mut self,
        _seat: &Seat<Self>,
        _focused: Option<&Self::KeyboardFocus>
    ) {}
}

delegate_seat!(Navda);

impl DataDeviceHandler for Navda {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

impl ClientDndGrabHandler for Navda {}
impl ServerDndGrabHandler for Navda {}

delegate_data_device!(Navda);

delegate_output!(Navda);