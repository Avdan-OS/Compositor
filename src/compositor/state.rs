//!
//! Hosts compositor's current state.
//! 

use std::{time::{Instant, Duration}, ffi::OsString, sync::{Arc, atomic::AtomicBool, Mutex}, os::unix::prelude::AsRawFd, process::Command};

use slog::Logger;
use smithay::{
    desktop::{
        Window, Space, WindowSurfaceType, utils::{update_surface_primary_scanout_output, surface_primary_scanout_output, OutputPresentationFeedback, surface_presentation_feedback_flags_from_states}
    },
    reexports::{
        calloop::{
            LoopSignal, EventLoop, 
            generic::Generic, Interest,
            Mode, PostAction, LoopHandle
        },
        wayland_server::{
            Display, protocol::wl_surface::WlSurface,
            backend::{
                ClientData, ClientId, DisconnectReason
            }
        },
    },
    wayland::{
        compositor::CompositorState, shell::xdg::XdgShellState,
        shm::ShmState, output::OutputManagerState,
        data_device::DataDeviceState, socket::ListeningSocketSource, fractional_scale::with_fractional_scale
    },
    input::{
        SeatState, Seat, pointer::{PointerHandle, CursorImageStatus}
    },
    utils::{Point, Logical, Clock, Monotonic}, output::Output, backend::renderer::element::{RenderElementStates, default_primary_scanout_output_compare}
};

use super::CalloopData;

///
/// State of the compositor.
/// 
pub struct Navda<BEnd : 'static> {
    pub clock       : Clock<Monotonic>,

    /// Wayland server socket name.
    pub socket_name : OsString,

    /// 2D Plane where all windows live.
    pub space       : Space<Window>,

    /// Compositor's logger.
    pub log         : Logger,

    pub backend_data: BEnd,

    /// @Sammy99jsp TODO: This description
    pub seat    : Seat<Self>,
    
    /// Are we actually running?
    pub running: Arc<AtomicBool>,

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
    pub show_window_preview : bool,

    /// Cursor status:
    /// 
    /// Either:
    /// * Hidden
    /// * Fully Compositor-drawn
    /// * Drawn to Surface
    /// 
    pub cursor_status       : Arc<Mutex<CursorImageStatus>>,

    ///
    /// Mouse Pointer's location on screen. 
    /// 
    pub pointer_location    : Point<i64, Logical>


}

impl<BEnd : 'static> Navda<BEnd> {
    /// Makes a new instance of the compositor.
    pub fn new(
        display     : &mut Display<Self>,
        handle      : LoopHandle<'static, CalloopData<BEnd>>,
        backend_data: BEnd,
        log         : Logger,
    ) -> Self {

        let clock = Clock::new().expect("Failed to initialize clock!");

        let socket_name = Self::init_wayland_listener(display, handle, log.clone());

        // Initialize global state members
        let dh = display.handle();
        let compositor_state = CompositorState::new::<Self, _>(&dh, log.clone());
        let xdg_shell_state = XdgShellState::new::<Self, _>(&dh, log.clone());
        let shm_state = ShmState::new::<Self, _>(&dh, vec![], log.clone());
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&dh);
        let data_device_state = DataDeviceState::new::<Self, _>(&dh, log.clone());
        let running = Arc::new(AtomicBool::new(true));
        

        let mut seat_state = SeatState::new();
        let mut seat : Seat<Self> = seat_state.new_wl_seat(&dh, "winit", log.clone());
        
        // TODO: Possible tablet cursor things here.
        //          we'll let the fine folks at Smithay
        //          do that :) 
        
        seat.add_keyboard(Default::default(), 200, 200).unwrap();
        
        // Mouse
        seat.add_pointer();
        let cursor_status = Arc::new(Mutex::new(CursorImageStatus::Default));
        let pointer_location = (0 ,0).into();
        // 2D area for windows to live
        let space = Space::new(log.clone());


        Self {
            cursor_status,
            pointer_location,
            space, clock, running,
            socket_name, backend_data,
            log, compositor_state,
            xdg_shell_state, shm_state,
            output_manager_state,
            seat_state, data_device_state,
            seat,
            show_window_preview: true,
        }
    }

    /// 
    /// Initializes the listener for Wayland events,
    /// and links it to the main event loop.
    ///  
    fn init_wayland_listener(
        display     : &mut Display<Self>,
        handle      : LoopHandle<CalloopData<BEnd>>,
        log         : Logger
    ) -> OsString {
        // Auto-magically chooses next available wayland socket name.
        let listening_socket = ListeningSocketSource::new_auto(log.clone())
            .unwrap();
        
        let socket_name = listening_socket.socket_name().to_os_string();


        handle
            .insert_source(listening_socket, move |client_stream , _, data| {
                if let Err(err) = data
                    .display
                    .handle()
                    .insert_client(client_stream, Arc::new(ClientState))
                {
                    slog::warn!(data.state.log, "Error adding Wayland client: {}", err);
                }
            })
            .expect("Failed to init Wayland event source.");

        slog::info!(
            log, 
            "Listening on Wayland socket source";
            "name" => socket_name.clone().to_str().unwrap()
        );
        ::std::env::set_var("WAYLAND_DISPLAY", &socket_name);

        Command::new("weston-terminal").spawn().unwrap();

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
            .expect("Failed to init Wayland server source.");

        
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

pub fn post_repaint(
    output  : &Output,
    render_element_states : &RenderElementStates,
    space   : &Space<Window>,
    time    : impl Into<Duration>
) {
    let time = time.into();
    let throttle = Some(Duration::from_secs(1));

    space.elements().for_each(|window| {
        window.with_surfaces(|surface, states| {
            let primary_scanout_output = update_surface_primary_scanout_output(
                surface,
                output,
                states,
                render_element_states,
                default_primary_scanout_output_compare,
            );

            if let Some(output) = primary_scanout_output {
                with_fractional_scale(states, |fraction_scale| {
                    fraction_scale.set_preferred_scale(output.current_scale().fractional_scale());
                });
            }
        });

        if space.outputs_for_element(window).contains(output) {
            window.send_frame(
                output, time,
                throttle, surface_primary_scanout_output
            );
        }
    });

    let map = smithay::desktop::layer_map_for_output(output);

    map.layers().for_each(|layer_surface| {
        layer_surface.with_surfaces(|surface, states| {
            let primary_scanout_output = update_surface_primary_scanout_output(
                surface,
                output,
                states,
                render_element_states,
                default_primary_scanout_output_compare,
            );
    
            if let Some(output) = primary_scanout_output {
                with_fractional_scale(states, |fraction_scale| {
                    fraction_scale.set_preferred_scale(
                        output.current_scale().fractional_scale()
                    );
                });
            }
        });
    
        layer_surface.send_frame(
            output, time, throttle, surface_primary_scanout_output
        );
    });
}

pub fn take_presentation_feedback(
    output  : &Output,
    space   : &Space<Window>,
    render_element_states : &RenderElementStates,
) -> OutputPresentationFeedback {
    let mut output_presentation_feedback = OutputPresentationFeedback::new(output);

    space.elements().for_each(|window| {
        if space.outputs_for_element(window).contains(output) {
            window.take_presentation_feedback(
                &mut output_presentation_feedback,
                surface_primary_scanout_output,
                |surface, _|
                            surface_presentation_feedback_flags_from_states(
                                surface, render_element_states
                            ),
            );
        }
    });
    let map = smithay::desktop::layer_map_for_output(output);
    for layer_surface in map.layers() {
        layer_surface.take_presentation_feedback(
            &mut output_presentation_feedback,
            surface_primary_scanout_output,
            |surface, _| 
                        surface_presentation_feedback_flags_from_states(
                            surface, render_element_states
                        ),
        );
    }

    output_presentation_feedback
}
///
/// State that's associated with a client.
/// 
pub struct ClientState;

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}