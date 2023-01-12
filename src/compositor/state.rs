//!
//! Hosts compositor's current state.
//! 

use std::{time::Instant, ffi::OsString, sync::Arc, os::unix::prelude::AsRawFd};

use slog::Logger;
use smithay::{desktop::{Window, Space, WindowSurfaceType}, reexports::{calloop::{LoopSignal, EventLoop, generic::Generic, Interest, Mode, PostAction}, wayland_server::{Display, protocol::wl_surface::WlSurface, backend::{ClientData, ClientId, DisconnectReason}}}, wayland::{compositor::CompositorState, shell::xdg::XdgShellState, shm::ShmState, output::OutputManagerState, data_device::DataDeviceState, socket::ListeningSocketSource}, input::{SeatState, Seat, pointer::PointerHandle}, utils::{Point, Logical}};

use super::CalloopData;

///
/// State of the compositor.
/// 
pub struct Navda {
    /// Time the compositor was started.
    pub start_time  : Instant,

    /// Wayland server socket name.
    pub socket_name : OsString,

    
    /// 2D Plane where all windows live.
    pub space       : Space<Window>,

    /// Control signal for the compositor's event loop.
    pub loop_signal : LoopSignal,

    /// Compositor's logger.
    pub log         : Logger,


    /// Smithay's state
    pub compositor_state    : CompositorState,
    
    /// Shell's global state -- list of surfaces.
    pub xdg_shell_state     : XdgShellState,
    
    /// @Sammy99jsp TODO: This description
    pub shm_state           : ShmState,

    /// Smithay's output manager.
    pub output_manager_state: OutputManagerState,

    /// @Sammy99jsp TODO: This description
    pub seat_state          : SeatState<Self>,

    /// @Sammy99jsp TODO: This description
    pub data_device_state   : DataDeviceState,

    /// @Sammy99jsp TODO: This description
    pub seat    : Seat<Self>,
}

impl Navda {
    /// Makes a new instance of the compositor.
    pub fn new(
        event_loop  : &mut EventLoop<CalloopData>,
        display     : &mut Display<Self>,
        log         : Logger,
    ) -> Self {
        let start_time = Instant::now();

        let dh = display.handle();

        let compositor_state = CompositorState::new::<Self, _>(&dh, log.clone());
        let xdg_shell_state = XdgShellState::new::<Self, _>(&dh, log.clone());
        let shm_state = ShmState::new::<Self, _>(&dh, vec![], log.clone());
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&dh);
        let mut seat_state = SeatState::new();
        let data_device_state = DataDeviceState::new::<Self, _>(&dh, log.clone());


        let mut seat : Seat<Self> = seat_state.new_wl_seat(&dh, "winit", log.clone());


        seat.add_keyboard(Default::default(), 200, 200).unwrap();

        // Mouse
        seat.add_pointer();

        // 2D area for windows to live
        let space = Space::new(log.clone());

        let socket_name = Self::init_wayland_listener(display, event_loop, log.clone());

        let loop_signal = event_loop.get_signal();

        Self {
            start_time, space,
            loop_signal, socket_name,
            log, compositor_state,
            xdg_shell_state, shm_state,
            output_manager_state,
            seat_state, data_device_state,
            seat,
        }
    }

    /// 
    /// Initializes the listener for Wayland events,
    /// and links it to the main event loop.
    ///  
    fn init_wayland_listener(
        display     : &mut Display<Navda>,
        event_loop  : &mut EventLoop<CalloopData>,
        log         : Logger
    ) -> OsString {
        // Auto-magically chooses next available wayland socket name.
        let listening_socket = ListeningSocketSource::new_auto(log).unwrap();
        
        let socket_name = listening_socket.socket_name().to_os_string();

        let handle = event_loop.handle();

        event_loop
            .handle()
            .insert_source(listening_socket, move |client_stream , _, state| {
                state
                    .display
                    .handle()
                    .insert_client(client_stream, Arc::new(ClientState))
                    .unwrap();
            })
            .expect("Failed to init Wayland event source.");

        handle
            .insert_source(
                Generic::new(
                    display.backend().poll_fd().as_raw_fd(),
                    Interest::READ, 
                    Mode::Level
                ), 
                |_, _, state| {
                    state.display.dispatch_clients(&mut state.state).unwrap();
                    Ok(PostAction::Continue)
                },
            )
            .unwrap();

        socket_name
    }

    ///
    /// Returns the surface (if any) under
    /// the users pointer. 
    /// 
    pub fn surface_under_pointer(
        &self,
        pointer : &PointerHandle<Self>,
    ) -> Option<(WlSurface, Point<i32, Logical>)> {
        let pos = pointer.current_location();
        self.space.element_under(pos)
            .and_then(|(window, location)| {
                window
                    .surface_under(
                        pos - location.to_f64(),
                        WindowSurfaceType::ALL
                    )
                    .map(|(s, p)| (s, p + location))
            })
    }
}

///
/// State that's associated with a client.
/// 
pub struct ClientState;

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}