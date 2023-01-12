use smithay::{
    wayland::{
        shell::xdg::{
            XdgShellHandler, XdgShellState, ToplevelSurface,
            PopupSurface, PositionerState, XdgToplevelSurfaceData
        },
        compositor::with_states
    },
    reexports::{
        wayland_server::{
            protocol::{
                wl_seat::WlSeat, wl_surface::WlSurface
            },
            Resource
        },
        wayland_protocols::xdg::shell::server::xdg_toplevel::{
            ResizeEdge, self
        }
    },
    utils::{
        Serial, Rectangle
    },
    desktop::{
        Window, Kind, Space
    },
    input::{
        Seat,
        pointer::{
            Focus, GrabStartData as PointerGrabStartData
        },
    },
    delegate_xdg_shell
};

use crate::compositor::{state::Navda, grabs::{MoveSurfaceGrab, ResizeSurfaceGrab}};

impl XdgShellHandler for Navda {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(
        &mut self,
        surface: ToplevelSurface
    ) {
        let window = Window::new(Kind::Xdg(surface));
        self.space.map_element(window, (0, 0), false);
    }

    fn new_popup(
        &mut self,
        surface: PopupSurface,
        positioner: PositionerState
    ) {
        // TODO: Popup handling using PopupManager
    }

    fn move_request(
        &mut self,
        surface: ToplevelSurface,
        seat: WlSeat,
        serial: Serial
    ) {
        let seat = Seat::from_resource(&seat).unwrap();
        
        let wl_surface = surface.wl_surface();

        if let Some(start_data) = check_grab(&seat, wl_surface, serial) {
            let pointer = seat.get_pointer().unwrap();

            let window = self
                .space
                .elements()
                .find(|w| w.toplevel().wl_surface() == wl_surface)
                .unwrap()
                .clone();
            
            let initial_window_location = self.space
                .element_location(&window).unwrap();
            
            let grab = MoveSurfaceGrab {
                start_data,
                window,
                initial_window_location
            };

            pointer.set_grab(self, grab, serial, Focus::Clear);
        }
    }

    fn resize_request(
        &mut self,
        surface: ToplevelSurface,
        seat: WlSeat,
        serial: Serial,
        edges: ResizeEdge,
    ) {
       let seat = Seat::from_resource(&seat).unwrap();
       
       let wl_surface = surface.wl_surface();

       if let Some(start_data) = check_grab(&seat, wl_surface, serial) {
        let pointer = seat.get_pointer().unwrap();

        let window = self
            .space.elements()
            .find(|w| w.toplevel().wl_surface() == wl_surface)
            .unwrap()
            .clone();

        let initial_window_location = self.space
            .element_location(&window).unwrap();
        let initial_window_size = window.geometry().size;

        surface.with_pending_state(|state| {
            state.states.set(xdg_toplevel::State::Resizing);
        });

        surface.send_configure();

        let grab = ResizeSurfaceGrab::start(
            start_data,
            window,
            edges.into(),
            Rectangle::from_loc_and_size(
                initial_window_location, initial_window_size
            )
        );

        pointer.set_grab(self, grab, serial, Focus::Clear);
       }
    }

    fn grab(
        &mut self,
        surface: PopupSurface,
        seat: WlSeat,
        serial: Serial
    ) {
        // TODO: Popup grabs
    }
}

delegate_xdg_shell!(Navda);

fn check_grab(
    seat    : &Seat<Navda>,
    surface : &WlSurface,
    serial  : Serial,
) -> Option<PointerGrabStartData<Navda>> {
    let ptr = seat.get_pointer()?;

    if !ptr.has_grab(serial) {
        return None;
    }

    let start_data = ptr.grab_start_data()?;

    let (focus, _) = start_data.focus.as_ref()?;

    if !focus.id().same_client_as(&surface.id()) {
        return None;
    }

    Some(start_data)
}

/// Called on `WlSurface::commit`
pub fn handle_commit(
    space   : &Space<Window>,
    surface : &WlSurface,
) -> Option<()> {
    let window = space
        .elements()
        .find(|w| w.toplevel().wl_surface() == surface)
        .cloned()?;

    if let Kind::Xdg(_) = window.toplevel() {
        let initial_configure_sent = with_states(
            surface,
            |states| {
                states
                    .data_map
                    .get::<XdgToplevelSurfaceData>()
                    .unwrap()
                    .lock()
                    .unwrap()
                    .initial_configure_sent
            }
        );

        if !initial_configure_sent {
            window.configure();
        }
    }

    Some(())
}