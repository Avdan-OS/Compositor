//!
//! Collection of XDG handlers/Smithay delegations:
//!
//! * [xdg_activation](https://wayland.app/protocols/xdg-activation-v1)
//! * [xdg_shell](https://wayland.app/protocols/xdg-shell)
//!
//!
//!
use smithay::{
    delegate_xdg_activation, delegate_xdg_decoration,
    reexports::{
        wayland_protocols::xdg::decoration::zv1::server::zxdg_toplevel_decoration_v1::Mode,
        wayland_server::protocol::wl_surface::WlSurface,
    },
    wayland::{
        compositor::with_states,
        shell::xdg::{decoration::XdgDecorationHandler, ToplevelSurface, XdgToplevelSurfaceData},
        xdg_activation::{
            XdgActivationHandler, XdgActivationState, XdgActivationToken, XdgActivationTokenData,
        },
    },
};

use crate::compositor::{backend::Backend, shell::avwindow::AvWindow, state::Navda};

impl<BEnd: Backend> XdgActivationHandler for Navda<BEnd> {
    fn activation_state(&mut self) -> &mut XdgActivationState {
        &mut self.xdg_activation_state
    }

    fn request_activation(
        &mut self,
        token: XdgActivationToken,
        token_data: XdgActivationTokenData,
        surface: WlSurface,
    ) {
        if token_data.timestamp.elapsed().as_secs() < 10 {
            // Just grant the wish
            let w = self
                .space
                .elements()
                .find(|window| window.wl_surface().map(|s| s == surface).unwrap_or(false))
                .cloned();
            if let Some(window) = w {
                self.space.raise_element(&window, true);
            }
        } else {
            // Discard the request
            self.xdg_activation_state.remove_request(&token);
        }
    }

    fn destroy_activation(
        &mut self,
        _token: XdgActivationToken,
        _token_data: XdgActivationTokenData,
        _surface: WlSurface,
    ) {
        // The request is cancelled
    }
}
delegate_xdg_activation!(@<BEnd: Backend + 'static> Navda<BEnd>);

impl<BEnd: Backend> XdgDecorationHandler for Navda<BEnd> {
    fn new_decoration(&mut self, toplevel: ToplevelSurface) {
        toplevel.with_pending_state(|state| {
            state.decoration_mode = Some(Mode::ClientSide);
        });
        toplevel.send_configure();
    }
    fn request_mode(&mut self, toplevel: ToplevelSurface, mode: Mode) {
        if let Some(w) = self
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
