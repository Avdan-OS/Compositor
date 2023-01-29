//!
//! XDG-related implementations:
//! Description
//! `handlers` &mdash; XDG Protocol Handlers
//! `decoration` &mdash; XD
//!

use std::cell::RefCell;

use smithay::{
    desktop::{PopupKind, Window},
    input::Seat,
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel::ResizeEdge,
        wayland_server::{
            protocol::wl_seat::{self, WlSeat},
            Resource,
        },
    },
    utils::Serial,
    wayland::{
        compositor::{with_states},
        shell::xdg::{
            PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
        },
    },
};

use crate::compositor::{backend::Backend, state::Navda};

mod decoration;
mod handlers;

impl<BEnd: Backend> XdgShellHandler for Navda<BEnd> {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = AvWindow::Wayland(Window::new(surface));
        place_new_window(&mut self.space, &window, true);
    }

    fn new_popup(&mut self, surface: PopupSurface, positioner: PositionerState) {
        surface.with_pending_state(|state| {
            state.geometry = positioner.get_geometry();
        });

        if let Err(err) = self.popups.track_popup(PopupKind::from(surface)) {
            slog::warn!(self.log, "Failed to track popup: {}", err);
        }
    }

    ///
    /// Respond to a reposition request:
    ///
    /// https://wayland.app/protocols/xdg-shell#xdg_popup:request:reposition
    ///
    fn reposition_request(
        &mut self,
        surface: PopupSurface,
        positioner: PositionerState,
        token: u32,
    ) {
        surface.with_pending_state(|state| {
            // TODO(@Sammy99jsp) Reference the follwing `anvil` comment:
            //     "This is again a simplification, a proper compositor would
            //      calculate the geometry of the popup here. For simplicity we just
            //      use the default implementation here that does not take the
            //      window position and output constraints into account."
            let geo = positioner.get_geometry();
            state.geometry = geo;
            state.positioner = positioner;
        });
        surface.send_repositioned(token);
    }

    fn move_request(&mut self, surface: ToplevelSurface, seat: wl_seat::WlSeat, serial: Serial) {
        let seat: Seat<Self> = Seat::from_resource(&seat).unwrap();
        self.move_request_xdg(&surface, &seat, serial);
    }

    ///
    /// User-driven interactive surface resize.
    ///
    /// https://wayland.app/protocols/xdg-shell#xdg_toplevel:request:resize
    ///
    fn resize_request(
        &mut self,
        surface: ToplevelSurface,
        seat: wl_seat::WlSeat,
        serial: Serial,
        edges: ResizeEdge,
    ) {
        let seat: Seat<Self> = Seat::from_resource(&seat).unwrap();
        // Anvil TODO: Touch resize
        let pointer = seat.get_pointer().unwrap();

        // Check if this surface has click grab.
        if !pointer.has_grab(serial) {
            return;
        }

        let start_data = pointer.grab_start_data().unwrap();

        let window = self.window_for_surface(surface.wl_surface()).unwrap();

        if start_data.focus.is_none()
            || !start_data
                .focus
                .as_ref()
                .unwrap()
                .0
                .same_client_as(&surface.wl_surface().id())
        {
            return;
        }

        let geo = window.geometry();
        let loc = self.space.element_location(&window).unwrap();
        let (initial_window_location, initial_window_size) = (loc, geo.size);

        with_states(surface.wl_surface(), move |states| {
            states
                .data_map
                .get::<RefCell<SurfaceData>>()
                .unwrap()
                .borrow()
        })
    }

    fn grab(&mut self, surface: PopupSurface, seat: WlSeat, serial: Serial) {
        todo!()
    }
}
