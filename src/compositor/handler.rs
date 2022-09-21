#![allow(unused_parens)]

use crate::{
    grabs::{
        MoveSurfaceGrab,
        ResizeSurfaceGrab,
    },
    AvCompositor,
};

use smithay::{
    backend::renderer::utils::on_commit_buffer_handler,
    delegate_compositor,
    delegate_data_device,
    delegate_output,
    delegate_seat,
    delegate_shm,
    delegate_xdg_shell,
    desktop::{
        Kind,
        Space,
        Window,
        WindowSurfaceType,
    },
    input::{
        pointer::{
            Focus,
            GrabStartData as PointerGrabStartData, PointerHandle,
        },
        Seat,
        SeatHandler,
        SeatState,
    },
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::{
            protocol::{
                wl_buffer,
                wl_seat,
                wl_surface::WlSurface,
            },
            Resource,
        },
    },
    utils::{
        Point,
        Logical,
        Rectangle,
        Serial,
        Size,
    },
    wayland::{
        buffer::BufferHandler,
        compositor::{
            CompositorHandler,
            CompositorState,
            with_states, SurfaceData,
        },
        data_device::{
            ClientDndGrabHandler,
            DataDeviceHandler,
            DataDeviceState,
            ServerDndGrabHandler,
        },
        shm::{
            ShmHandler,
            ShmState,
        },
        shell::xdg::{
            PopupSurface,
            PositionerState,
            ToplevelSurface,
            XdgShellHandler,
            XdgShellState,
            XdgToplevelSurfaceData, ToplevelState,
        },
    },
};

impl SeatHandler for AvCompositor {
    type KeyboardFocus = WlSurface;
    type PointerFocus  = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<AvCompositor> {
        &mut self.seat_state
    }

    fn cursor_image (
        &mut self,
        _seat : &smithay::input::Seat<Self>,
        _image: smithay::input::pointer::CursorImageStatus,
    ) {}

    fn focus_changed (
        &mut self,
        _seat   : &smithay::input::Seat<Self>,
        _focused: Option<&WlSurface>
    ) {}
}

delegate_seat!(AvCompositor);

impl DataDeviceHandler for AvCompositor {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

impl ClientDndGrabHandler for AvCompositor {}
impl ServerDndGrabHandler for AvCompositor {}

delegate_data_device!(AvCompositor);

delegate_output!(AvCompositor);

impl CompositorHandler for AvCompositor {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler(surface);
        self.space.commit(surface);

        handle_commit(&self.space, surface);
        handle_commit(&mut self.space, surface);
    }
}

impl BufferHandler for AvCompositor {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl ShmHandler for AvCompositor {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

delegate_compositor!(AvCompositor);
delegate_shm!(AvCompositor);

impl XdgShellHandler for AvCompositor {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel(&mut self, surface: ToplevelSurface) {
        let window: Window = Window::new(Kind::Xdg(surface));

        self.space.map_window(&window, (0, 0), None, false);
    }

    fn new_popup(&mut self, _surface: PopupSurface, _positioner: PositionerState) {
        // TODO: Popup handling using PopupManager
    }

    fn move_request (
        &mut self,
        surface: ToplevelSurface,
        seat   : wl_seat::WlSeat,
        serial : Serial
    ) {
        let seat: Seat<AvCompositor> = Seat::from_resource(&seat).unwrap();

        let wl_surface: &WlSurface = surface.wl_surface();

        if let Some(start_data) = check_grab(&seat, wl_surface, serial) {
            let pointer: PointerHandle<AvCompositor> = seat.get_pointer().unwrap();

            let window: Window = self
                .space
                .window_for_surface(wl_surface, WindowSurfaceType::TOPLEVEL)
                .unwrap()
                .clone();
            let initial_window_location: Point<i32, Logical> = self.space.window_location(&window).unwrap();

            let grab: MoveSurfaceGrab = MoveSurfaceGrab {
                start_data,
                window,
                initial_window_location,
            };

            pointer.set_grab(self, grab, serial, Focus::Clear);
        }
    }

    fn resize_request (
        &mut self,
        surface: ToplevelSurface,
        seat   : wl_seat::WlSeat,
        serial : Serial,
        edges  : xdg_toplevel::ResizeEdge,
    ) {
        let seat: Seat<AvCompositor> = Seat::from_resource(&seat).unwrap();

        let wl_surface: &WlSurface = surface.wl_surface();

        if let Some(start_data) = check_grab(&seat, wl_surface, serial) {
            let pointer: PointerHandle<AvCompositor> = seat.get_pointer().unwrap();

            let window: Window = self
                .space
                .window_for_surface(wl_surface, WindowSurfaceType::TOPLEVEL)
                .unwrap()
                .clone();
            let initial_window_location: Point<i32, Logical> = self.space.window_location(&window).unwrap();
            let initial_window_size    : Size<i32, Logical> = window.geometry().size;

            surface.with_pending_state(|state: &mut ToplevelState| {
                state.states.set(xdg_toplevel::State::Resizing);
            });

            surface.send_configure();

            let grab: ResizeSurfaceGrab = ResizeSurfaceGrab::start (
                start_data,
                window,
                edges.into(),
                Rectangle::from_loc_and_size(initial_window_location, initial_window_size),
            );

            pointer.set_grab(self, grab, serial, Focus::Clear);
        }
    }

    fn grab (
        &mut self,
        _surface: PopupSurface,
        _seat: wl_seat::WlSeat,
        _serial: Serial
    ) { /* TODO popup grabs */ }
}

// Xdg Shell
delegate_xdg_shell!(AvCompositor);

fn check_grab (
    seat   : &Seat<AvCompositor>,
    surface: &WlSurface,
    serial : Serial,
) -> Option<PointerGrabStartData<AvCompositor>> {
    let pointer: PointerHandle<AvCompositor> = seat.get_pointer()?;

    if (!pointer.has_grab(serial)) {
        return None;
    }

    let start_data = pointer.grab_start_data()?;

    let (focus, _) = start_data.focus.as_ref()?;
    // If the focus was for a different surface, ignore the request.
    if (!focus.id().same_client_as(&surface.id())) {
        return None;
    }

    Some(start_data)
}

/// Should be called on `WlSurface::commit`
pub fn handle_commit(space: &Space, surface: &WlSurface) -> Option<()> {
    let window: Window = space
        .window_for_surface(surface, WindowSurfaceType::TOPLEVEL)
        .cloned()?;

    if let Kind::Xdg(_) = window.toplevel() {
        let initial_configure_sent: bool = with_states(surface, |states: &SurfaceData| {
            states
                .data_map
                .get::<XdgToplevelSurfaceData>()
                .unwrap()
                .lock()
                .unwrap()
                .initial_configure_sent
        });

        if (!initial_configure_sent) {
            window.configure();
        }
    }

    Some(())
}
