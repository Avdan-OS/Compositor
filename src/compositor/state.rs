use crate::CalloopData;

use slog::Logger;

use smithay::{
    desktop::{
        Space,
        WindowSurfaceType,
    },
    input::{
        pointer::PointerHandle,
        Seat,
        SeatState,
    },
    reexports::{
        calloop::{
            generic::Generic,
            EventLoop,
            Interest,
            LoopSignal,
            Mode, 
            PostAction, LoopHandle, Readiness,
        },
        wayland_server::{
            backend::{
                ClientData,
                ClientId,
                DisconnectReason,
            },
            Display,
            DisplayHandle,
            protocol::wl_surface::WlSurface,
        }, io_lifetimes::BorrowedFd,
    },
    utils::{
        Logical,
        Point,
    },
    wayland::{
        compositor::CompositorState,
        data_device::DataDeviceState,
        output::OutputManagerState,
        shell::xdg::XdgShellState,
        shm::ShmState, 
        socket::ListeningSocketSource,
    },
};

use std::{
    ffi::OsString,
    os::unix::{
        io::AsRawFd,
        net::UnixStream,
    },
    sync::Arc,
    time::Instant,
};

pub struct AvCompositor {
    pub start_time : std::time::Instant,
    pub socket_name: OsString,

    pub space      : Space,
    pub loop_signal: LoopSignal,
    pub log        : slog::Logger,

    // Smithay state
    pub compositor_state    : CompositorState,
    pub xdg_shell_state     : XdgShellState,
    pub shm_state           : ShmState,
    pub output_manager_state: OutputManagerState,
    pub seat_state          : SeatState<AvCompositor>,
    pub data_device_state   : DataDeviceState,

    pub seat: Seat<Self>,
}

impl AvCompositor {
    pub fn new (
        event_loop: &mut EventLoop<CalloopData>,
        display:    &mut Display<Self>,
        log:        Logger
    ) -> Self {
        let start_time: Instant = std::time::Instant::now();

        let dh: DisplayHandle = display.handle(); //: DisplayHandle

        let compositor_state    : CompositorState         = CompositorState::new::<Self, _>(&dh, log.clone());
        let xdg_shell_state     : XdgShellState           = XdgShellState::new::<Self, _>(&dh, log.clone());
        let shm_state           : ShmState                = ShmState::new::<Self, _>(&dh, vec![], log.clone());
        let output_manager_state: OutputManagerState      = OutputManagerState::new_with_xdg_output::<Self>(&dh);
        let mut seat_state      : SeatState<AvCompositor> = SeatState::new();
        let data_device_state   : DataDeviceState         = DataDeviceState::new::<Self, _>(&dh, log.clone());

        // A seat is a group of input devices.
        let mut seat: Seat<Self> = seat_state.new_wl_seat(&dh, "winit", log.clone());

        // Notify clients that we have a keyboard
        // TODO: Implement keyboard hot-plug tracking
        seat.add_keyboard(Default::default(), 200, 200)
            .unwrap();

        // Notify clients that we have a keyboard
        // TODO: Implement mouse hot-plug tracking
        seat.add_pointer();

        // A space represents a two-dimensional plane. Windows and Outputs can be mapped onto it.
        let space: Space = Space::new(log.clone());

        let socket_name: OsString = Self::init_wayland_listener(display, event_loop, log.clone());

        // Get the loop signal, used to stop the event loop
        let loop_signal: LoopSignal = event_loop.get_signal();

        Self {
            start_time,

            space,
            loop_signal,
            socket_name,

            log,
            compositor_state,
            xdg_shell_state,
            shm_state,
            output_manager_state,
            seat_state,
            data_device_state,
            seat,
        }
    }

    fn init_wayland_listener<'display, 'event_loop, 'a>(
        display:    &'display mut Display<AvCompositor>,
        event_loop: &'event_loop mut EventLoop<'a, CalloopData>,
        log:        Logger,
    ) -> OsString {
        // Creates a new listening socket, automatically choosing the next available `wayland` socket name.
        let listening_socket: ListeningSocketSource = ListeningSocketSource::new_auto(log).unwrap();

        // Get the name of the listening socket.
        // Clients will connect to this socket.
        let socket_name: OsString = listening_socket.socket_name().to_os_string();

        let handle: LoopHandle<CalloopData> = event_loop.handle();

        event_loop
            .handle()
            .insert_source (listening_socket, move |client_stream: UnixStream, _, state: &mut CalloopData| {
                // Inside the callback, you should insert the client into the display.
                //
                // You may also associate some data with the client when inserting the client.
                state
                    .display
                    .handle()
                    .insert_client(client_stream, Arc::new(ClientState))
                    .unwrap();
            })
            .expect("Failed to start the wayland event source!");

        // You also need to add the display itself to the event loop, so that client events will be processed by wayland-server.
        handle
            .insert_source (
                Generic::new (
                    display.backend().poll_fd().as_raw_fd(),
                    Interest::READ,
                    Mode::Level,
                ),
                |_, _, state: &mut CalloopData| {
                    state.display.dispatch_clients(&mut state.state).unwrap();
                    Ok(PostAction::Continue)
                },
            )
            .unwrap();

        socket_name
    }

    fn test<'a>(_ : Readiness, _ : &mut BorrowedFd, state : &'a mut CalloopData) -> std::result::Result<PostAction, std::io::Error> {
        let d = &mut state.display;
        let s = &mut state.state;
        d.dispatch_clients(s);
        Ok(PostAction::Continue)
    }

    pub fn surface_under_pointer (
        &self,
        pointer: &PointerHandle<Self>,
    ) -> Option<(WlSurface, Point<i32, Logical>)> {
        let pos: Point<f64, Logical> = pointer.current_location();
        self.space
            .surface_under(pos, WindowSurfaceType::all())
            .map(|(_, surface, location): (_, _, Point<i32, Logical>)| (surface, location))
    }
}

pub struct ClientState;
impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {
        println!("Client successfully initialized!")
    }

    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {
        println!("Client disconnected.")
    }
}
