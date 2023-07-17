//!
//! Server-Side Decorations
//!
//! This would be the place to introduce the Tabbing UI.
//!

use smithay::{
    delegate_xdg_decoration,
    reexports::wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode,
    wayland::{
        compositor::with_states,
        shell::xdg::{decoration::XdgDecorationHandler, ToplevelSurface, XdgToplevelSurfaceData},
    },
};

use crate::compositor::{backend::Backend, shell::AvWindow, state::Navda};

impl<BEnd: Backend> XdgDecorationHandler for Navda<BEnd> {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(Mode::ClientSide);
        });
        toplevel.send_configure();
    }
    fn request_mode(&mut self, toplevel: ToplevelSurface, mode: Mode) {
        if let Some(_w) = self
            .space
            .elements()
            .find(|window| matches!(window, AvWindow::Wayland(w) if w.toplevel() == &toplevel))
        {
            toplevel.with_pending_state(|state| {
                state.decoration_mode = Some(match mode {
                    Mode::ServerSide => {
                        // TODO(Sammy99jsp) Server-side decoration.
                        // w.set_ssd(true);
                        Mode::ServerSide
                    }
                    _ => {
                        // TODO(Sammy99jsp) Server-side decoration.
                        // w.set_ssd(false);
                        Mode::ClientSide
                    }
                });
            });

            let initial_configure_sent = with_states(toplevel.wl_surface(), |states| {
                states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .initial_configure_sent
            });
            if initial_configure_sent {
                toplevel.send_configure();
            }
        }
    }
    fn unset_mode(&mut self, toplevel: ToplevelSurface) {
        if let Some(w) = self
            .space
            .elements()
            .find(|window| matches!(window, AvWindow::Wayland(w) if w.toplevel() == &toplevel))
        {
            // w.set_ssd(false);
            toplevel.with_pending_state(|state| {
                state.decoration_mode = Some(Mode::ClientSide);
            });
            let initial_configure_sent = with_states(toplevel.wl_surface(), |states| {
                states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .initial_configure_sent
            });
            if initial_configure_sent {
                toplevel.send_configure();
            }
        }
    }
}
delegate_xdg_decoration!(@<BEnd: Backend + 'static> Navda<BEnd>);
