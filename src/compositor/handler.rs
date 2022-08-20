use std::os::unix::prelude::RawFd;

use smithay::{
    backend::renderer::utils::on_commit_buffer_handler,
    delegate_compositor,
    delegate_data_device,
    delegate_seat,
    delegate_shm,
    delegate_xdg_shell,
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::{
            backend::{
                ClientData,
                ClientId,
                DisconnectReason,
            },
            DisplayHandle,
            protocol::{
                wl_buffer,
                wl_seat,
                wl_surface::WlSurface,
            },
        },
    },
    wayland::{
        buffer::BufferHandler,
        compositor::{
            CompositorHandler,
            CompositorState,
        },
        data_device::{
            ClientDndGrabHandler,
            DataDeviceHandler,
            DataDeviceState,
            ServerDndGrabHandler,
        },
        seat::{
            Seat,
            SeatHandler,
            SeatState,
        },
        shell::xdg::{
            PopupSurface,
            PositionerState,
            ToplevelSurface,
            XdgShellHandler,
            XdgShellState, ToplevelState,
        },
        shm::{
            ShmHandler,
            ShmState,
        },
        Serial,
    },
};

impl BufferHandler for App {
    fn buffer_destroyed (&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl XdgShellHandler for App {
    fn xdg_shell_state(&mut self) -> &mut XdgShellState {
        &mut self.xdg_shell_state
    }

    fn new_toplevel (
        &mut self,
        _dh: &DisplayHandle,
        surface: ToplevelSurface
    ) {
        surface.with_pending_state(|state: &mut ToplevelState| {
            state.states.set(xdg_toplevel::State::Activated);
        });
        surface.send_configure();
    }

    fn new_popup (
        &mut self,
        _dh: &DisplayHandle,
        _surface: PopupSurface,
        _positioner: PositionerState
    ) {}

    fn grab (
        &mut self,
        _dh: &DisplayHandle,
        _surface: PopupSurface,
        _seat: wl_seat::WlSeat,
        _serial: Serial
    ) {}
}

impl DataDeviceHandler for App {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }

    fn send_selection (
        &mut self,
        _dh: &DisplayHandle,
        _mime_type: String,
        _fd: RawFd
    ) {}
}

impl ClientDndGrabHandler for App {}

impl ServerDndGrabHandler for App {
    fn send (
        &mut self,
        _mime_type: String,
        _fd: RawFd
    ) {}
}

impl CompositorHandler for App {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn commit (
        &mut self,
        _dh: &DisplayHandle,
        surface: &WlSurface
    ) {
        on_commit_buffer_handler(surface);
    }
}

impl ShmHandler for App {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }
}

pub struct App {
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub seat_state: SeatState<Self>,
    pub data_device_state: DataDeviceState,
    pub seat: Seat<Self>,
}

pub struct ClientState;
impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {
        println!("Client initialized.");
    }

    fn disconnected (
        &self,
        _client_id: ClientId,
        _reason: DisconnectReason
    ) {
        println!("Client disconnected.");
    }
}

// Macros used to delegate protocol handling to types in the app state.
delegate_xdg_shell!(App);
delegate_compositor!(App);
delegate_shm!(App);
delegate_seat!(App);
delegate_data_device!(App);
