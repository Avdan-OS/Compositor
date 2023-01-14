mod compositor;
mod xdg_shell;

use smithay::{
    input::{
        SeatHandler, Seat,
        pointer::CursorImageStatus, SeatState
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    delegate_seat, wayland::data_device::{DataDeviceHandler, DataDeviceState, ClientDndGrabHandler, ServerDndGrabHandler}, delegate_data_device, delegate_output
};

use super::state::Navda;

impl<BEnd : 'static> SeatHandler for Navda<BEnd> {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
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

delegate_seat!(@<BEnd : 'static> Navda<BEnd>);

impl<BEnd : 'static> DataDeviceHandler for Navda<BEnd> {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

impl<BEnd : 'static> ClientDndGrabHandler for Navda<BEnd> {}
impl<BEnd : 'static> ServerDndGrabHandler for Navda<BEnd> {}

delegate_data_device!(@<BEnd : 'static> Navda<BEnd>);

delegate_output!(@<BEnd : 'static> Navda<BEnd>);