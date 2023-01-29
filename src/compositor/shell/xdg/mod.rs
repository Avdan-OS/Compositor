//!
//! XDG-related implementations.
//!
//! See `impl XDGShellHandler for Navda<...>` for more information
//! on XDG Shell.  
//!

use std::cell::RefCell;

use smithay::{
    delegate_xdg_shell,
    desktop::{
        find_popup_root_surface, layer_map_for_output, space::SpaceElement, PopupKeyboardGrab,
        PopupKind, PopupPointerGrab, PopupUngrabStrategy, Window, WindowSurfaceType,
    },
    input::{pointer::Focus, Seat},
    output::Output,
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel::{self, ResizeEdge},
        wayland_server::{
            protocol::{
                wl_output,
                wl_seat::{self, WlSeat},
                wl_surface,
            },
            Resource,
        },
    },
    utils::{Rectangle, Serial},
    wayland::{
        compositor::with_states,
        seat::WaylandFocus,
        shell::xdg::{
            self, PopupSurface, PositionerState, ToplevelSurface, XdgShellHandler, XdgShellState,
            XdgToplevelSurfaceData,
        },
    },
};

use crate::compositor::{
    backend::Backend, focus::FocusTarget, shell::FullscreenSurface, state::Navda,
};

use super::{
    avwindow::AvWindow,
    fullscreen_output_geometry,
    grabs::{MoveSurfaceGrab, ResizeData, ResizeState, ResizeSurfaceGrab},
    place_new_window, SurfaceData,
};

mod decoration;
mod handlers;

///
/// Implementing the [XDG Shell](https://wayland.app/protocols/xdg-shell) protocol.
///
/// Essentially, the XDG Shell protocol describes most of the graphical
/// features of a typical desktop OS, such as windows and popups.
///
/// XDG Shell also provides events, and interfaces to handle
/// actions such as resizing and moving windows -- it is a protocol for
/// communicating between compositors and client applications.
///
impl<BEnd: Backend> XdgShellHandler for Navda<BEnd> {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window = AvWindow::Wayland(Window::new(surface));
        place_new_window(&mut self.space, &window, true);
    }

    ///
    /// [xdg_popup](https://wayland.app/protocols/xdg-shell#xdg_popup)
    ///
    ///
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
        // TODO(from Anvil): "Touch resize"
        let pointer = seat.get_pointer().unwrap();

        // Check if this surface has click grab.
        if !pointer.has_grab(serial) {
            return;
        }

        let start_data = pointer.grab_start_data().unwrap();

        let window = self.window_for_surface(surface.wl_surface()).unwrap();

        // If focus is on a different surface, ignore.
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
                .borrow_mut()
                .resize_state = ResizeState::Resizing(ResizeData {
                edges: edges.into(),
                initial_rect: Rectangle::from_loc_and_size(
                    initial_window_location,
                    initial_window_size,
                ),
            })
        });

        let grab = ResizeSurfaceGrab {
            start_data,
            window,
            edges: edges.into(),
            initial_rect: Rectangle::from_loc_and_size(
                initial_window_location,
                initial_window_size,
            ),
            last_window_size: initial_window_size,
        };

        pointer.set_grab(self, grab, serial, Focus::Clear);
    }

    fn ack_configure(&mut self, surface: wl_surface::WlSurface, configure: xdg::Configure) {
        if let xdg::Configure::Toplevel(configure) = configure {
            if let Some(serial) = with_states(&surface, |states| {
                if let Some(data) = states.data_map.get::<RefCell<SurfaceData>>() {
                    if let ResizeState::WaitingForFinalAck(_, serial) = data.borrow().resize_state {
                        return Some(serial);
                    }
                }

                None
            }) {
                // NOTE: From `anvil`:
                // When the resize grab is released the surface
                // resize state will be set to WaitingForFinalAck
                // and the client will receive a configure request
                // without the resize state to inform the client
                // resizing has finished. Here we will wait for
                // the client to acknowledge the end of the
                // resizing. To check if the surface was resizing
                // before sending the configure we need to use
                // the current state as the received acknowledge
                // will no longer have the resize state set
                let is_resizing = with_states(&surface, |states| {
                    states
                        .data_map
                        .get::<XdgToplevelSurfaceData>()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .current
                        .states
                        .contains(xdg_toplevel::State::Resizing)
                });

                if configure.serial >= serial && is_resizing {
                    with_states(&surface, |states| {
                        let mut data = states
                            .data_map
                            .get::<RefCell<SurfaceData>>()
                            .unwrap()
                            .borrow_mut();
                        if let ResizeState::WaitingForFinalAck(resize_data, _) = data.resize_state {
                            data.resize_state = ResizeState::WaitingForCommit(resize_data);
                        } else {
                            unreachable!()
                        }
                    });
                }
            }
        }
    }

    fn fullscreen_request(
        &mut self,
        surface: ToplevelSurface,
        mut wl_output: Option<wl_output::WlOutput>,
    ) {
        // TODO(Sammy99jsp) review comment from `anvil`:
        // NOTE: This is only one part of the solution. We can set the
        // location and configure size here, but the surface should be rendered fullscreen
        // independently from its buffer size
        let wl_surface = surface.wl_surface();

        let output_geometry =
            fullscreen_output_geometry(wl_surface, wl_output.as_ref(), &mut self.space);

        if let Some(geometry) = output_geometry {
            let output = wl_output
                .as_ref()
                .and_then(Output::from_resource)
                .unwrap_or_else(|| self.space.outputs().next().unwrap().clone());
            let client = self.display_handle.get_client(wl_surface.id()).unwrap();
            for output in output.client_outputs(&client) {
                wl_output = Some(output);
            }
            let window = self
                .space
                .elements()
                .find(|window| {
                    window
                        .wl_surface()
                        .map(|s| s == *wl_surface)
                        .unwrap_or(false)
                })
                .unwrap();

            surface.with_pending_state(|state| {
                state.states.set(xdg_toplevel::State::Fullscreen);
                state.size = Some(geometry.size);
                state.fullscreen_output = wl_output;
            });
            surface.send_configure();
            output
                .user_data()
                .insert_if_missing(FullscreenSurface::default);
            output
                .user_data()
                .get::<FullscreenSurface>()
                .unwrap()
                .set(window.clone());
            slog::trace!(self.log, "Fullscreening: {:?}", window);
        }
    }

    fn unfullscreen_request(&mut self, surface: ToplevelSurface) {
        let ret = surface.with_pending_state(|state| {
            state.states.unset(xdg_toplevel::State::Fullscreen);
            state.size = None;
            state.fullscreen_output.take()
        });
        if let Some(output) = ret {
            let output = Output::from_resource(&output).unwrap();
            if let Some(fullscreen) = output.user_data().get::<FullscreenSurface>() {
                slog::trace!(self.log, "Unfullscreening: {:?}", fullscreen.get());
                fullscreen.clear();
                self.backend_data.reset_buffers(&output);
            }

            surface.send_configure();
        }
    }

    fn maximize_request(&mut self, surface: ToplevelSurface) {
        // NOTE from `anvil`: This should use layer-shell when it is implemented to
        // get the correct maximum size
        let window = self.window_for_surface(surface.wl_surface()).unwrap();
        let outputs_for_window = self.space.outputs_for_element(&window);
        let output = outputs_for_window
            .first()
            // The window hasn't been mapped yet, use the primary output instead
            .or_else(|| self.space.outputs().next())
            // Assumes that at least one output exists
            .expect("No outputs found");
        let geometry = self.space.output_geometry(output).unwrap();

        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Maximized);
            state.size = Some(geometry.size);
        });
        surface.send_configure();
        self.space.map_element(window, geometry.loc, true);
    }

    fn unmaximize_request(&mut self, surface: ToplevelSurface) {
        surface.with_pending_state(|state| {
            state.states.unset(xdg_toplevel::State::Maximized);
            state.size = None;
        });
        surface.send_configure();
    }

    fn grab(&mut self, surface: PopupSurface, seat: WlSeat, serial: Serial) {
        let seat: Seat<Navda<BEnd>> = Seat::from_resource(&seat).unwrap();
        let kind = PopupKind::Xdg(surface);
        if let Some(root) = find_popup_root_surface(&kind).ok().and_then(|root| {
            self.space
                .elements()
                .find(|w| w.wl_surface().map(|s| s == root).unwrap_or(false))
                .cloned()
                .map(FocusTarget::Window)
                .or_else(|| {
                    self.space
                        .outputs()
                        .find_map(|o| {
                            let map = layer_map_for_output(o);
                            map.layer_for_surface(&root, WindowSurfaceType::TOPLEVEL)
                                .cloned()
                        })
                        .map(FocusTarget::LayerSurface)
                })
        }) {
            let ret = self.popups.grab_popup(root, kind, &seat, serial);

            if let Ok(mut grab) = ret {
                if let Some(keyboard) = seat.get_keyboard() {
                    if keyboard.is_grabbed()
                        && !(keyboard.has_grab(serial)
                            || keyboard.has_grab(grab.previous_serial().unwrap_or(serial)))
                    {
                        grab.ungrab(PopupUngrabStrategy::All);
                        return;
                    }
                    keyboard.set_focus(self, grab.current_grab(), serial);
                    keyboard.set_grab(PopupKeyboardGrab::new(&grab), serial);
                }
                if let Some(pointer) = seat.get_pointer() {
                    if pointer.is_grabbed()
                        && !(pointer.has_grab(serial)
                            || pointer
                                .has_grab(grab.previous_serial().unwrap_or_else(|| grab.serial())))
                    {
                        grab.ungrab(PopupUngrabStrategy::All);
                        return;
                    }
                    pointer.set_grab(self, PopupPointerGrab::new(&grab), serial, Focus::Keep);
                }
            }
        }
    }
}

delegate_xdg_shell!(@<BEnd : Backend + 'static> Navda<BEnd>);

impl<BEnd: Backend> Navda<BEnd> {
    fn move_request_xdg(&mut self, surface: &ToplevelSurface, seat: &Seat<Self>, serial: Serial) {
        // TODO: touch move.
        let pointer = seat.get_pointer().unwrap();

        // Check that this surface has a click grab.
        if !pointer.has_grab(serial) {
            return;
        }

        let start_data = pointer.grab_start_data().unwrap();

        let window = self.window_for_surface(surface.wl_surface()).unwrap();

        // If the focus was for a different surface, ignore the request.
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

        let mut initial_window_location = self.space.element_location(&window).unwrap();

        // If surface is maximized then unmaximize it
        let current_state = surface.current_state();
        if current_state
            .states
            .contains(xdg_toplevel::State::Maximized)
        {
            surface.with_pending_state(|state| {
                state.states.unset(xdg_toplevel::State::Maximized);
                state.size = None;
            });

            surface.send_configure();

            // TODO(Sammy99jsp) review below:
            // NOTE: In real compositor mouse location should be mapped to a new window size
            // For example, you could:
            // 1) transform mouse pointer position from compositor space to window space (location relative)
            // 2) divide the x coordinate by width of the window to get the percentage
            //   - 0.0 would be on the far left of the window
            //   - 0.5 would be in middle of the window
            //   - 1.0 would be on the far right of the window
            // 3) multiply the percentage by new window width
            // 4) by doing that, drag will look a lot more natural
            //
            // but for anvil needs setting location to pointer location is fine
            let pos = pointer.current_location();
            initial_window_location = (pos.x as i32, pos.y as i32).into();
        }

        let grab = MoveSurfaceGrab {
            start_data,
            window,
            initial_window_location,
        };

        pointer.set_grab(self, grab, serial, Focus::Clear);
    }
}
