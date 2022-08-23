use crate::compositor::handler::Compositor;
use crate::compositor::state::ClientState;

use std::{
    error::Error,
    sync::Arc,
    time::Instant,
};

use smithay::{
    backend::{ // helpers for interaction with the operating system (session management, input handling, and graphics.)
        input::{ // common traits for input backends to receive input from
            InputEvent,
            KeyboardKeyEvent,
        },
        renderer::{ // collection of common traits and implementations to facilitate rendering
            Frame,
            gles2::{
                Gles2Renderer,
                Gles2Frame,
            },
            Renderer,
            utils::draw_surface_tree,
        },
        winit::{ // implementations of the backend interfaces for running a compositor as a Wayland of X11 client using winit
            self,
            WinitEvent, WinitGraphicsBackend, WinitEventLoop,
        },
    },
    desktop::utils::send_frames_surface_tree,
    reexports::wayland_server::{ // interface for interacting with the Wayland protocol, server-side.
        Client,
        Display,
        DisplayHandle,
        ListeningSocket,
        protocol::wl_surface::WlSurface,
    },
    utils::{  // smithay utilities (mostly coordinate space stuff)
        Physical,
        Rectangle,
        Size,
        Transform,
    },
    wayland::{ // handlers to manage the Wayland protocol and the clients
        compositor::CompositorState,
        data_device::DataDeviceState,
        seat::{ // utilities for handling the seat globals and the associated input Wayland objects
            FilterResult,
            KeyboardHandle,
            Seat,
            SeatState,
        },
        shm::ShmState,
    },
};

use smithay::wayland::shell::xdg::{ // handler of utilities for the various shell protocols
            ToplevelSurface,
            XdgShellState,
        };

use slog::o;

fn log() -> ::slog::Logger {
    ::slog::Logger::root(
        ::slog_scope::logger(),
        o!(),
    )
}

pub fn init_winit() -> Result<(), Box<dyn Error>> {
    let log: ::slog::Logger = log();

    let mut display: Display<Compositor> = Display::new()
        .unwrap();
    let dh: DisplayHandle = display.handle();

    let seat_state: SeatState<Compositor> = SeatState::new();
    let seat: Seat<Compositor> = Seat::<Compositor>::new(&dh, "winit", None);

    let mut state: Compositor = {
        Compositor {
            compositor_state: CompositorState::new::<Compositor, _>(&dh, None),
            xdg_shell_state: XdgShellState::new::<Compositor, _>(&dh, None),
            shm_state: ShmState::new::<Compositor, _>(&dh, vec![], None),
            seat_state,
            data_device_state: DataDeviceState::new::<Compositor, _>(&dh, None),
            seat,
        }
    };

    let listener: ListeningSocket = ListeningSocket::bind("wayland-5").unwrap();
    let mut clients: Vec<Client> = Vec::new();

    let (mut backend, mut winit): (WinitGraphicsBackend, WinitEventLoop) =
        winit::init(None)
            .unwrap();

    let start_time: Instant = std::time::Instant::now();

    let keyboard: KeyboardHandle = state
        .seat
        .add_keyboard (
            Default::default(),
            200,
            200,
            |_, _| {}
        ).unwrap();

    std::env::set_var("WAYLAND_DISPLAY", "wayland-5");
    std::process::Command::new("weston-terminal").spawn().ok();

    loop {
        winit.dispatch_new_events(|event: WinitEvent| match event {
            WinitEvent::Resized { .. } => {}
            
            WinitEvent::Input(event) => match event {
                InputEvent::Keyboard { event } => {
                    let dh: &mut DisplayHandle = &mut display.handle();

                    keyboard.input::<(), _> (
                            dh,
                            event.key_code(),
                            event.state(),
                            0.into(),
                            0,
                            |_, _| {
                        FilterResult::Forward
                    });
                }

                InputEvent::PointerMotionAbsolute { .. } => {
                    let dh: &mut DisplayHandle = &mut display.handle();
                    
                    state.xdg_shell_state.toplevel_surfaces(|surfaces: &[ToplevelSurface]| {
                        if let Some(surface) = surfaces.iter().next() {
                            let surface: &WlSurface = surface.wl_surface();

                            keyboard.set_focus(dh, Some(surface), 0.into());
                        }
                    });
                }

                _ => {}
            },
            
            _ => (),
        })?;

        backend.bind().unwrap();

        let size: Size<i32, Physical> = backend.window_size().physical_size;
        let damage: Rectangle<i32, Physical> = Rectangle::from_loc_and_size((0, 0), size);

        backend
            .renderer()
            .render(
                size,
                Transform::Flipped180,
                |renderer: &mut Gles2Renderer,
                           frame: &mut Gles2Frame| {
                frame.clear([0.1, 0.0, 0.0, 1.0], &[damage]).unwrap();

                state.xdg_shell_state.toplevel_surfaces(|surfaces: &[ToplevelSurface]| {
                    for surface in surfaces {
                        let surface: &WlSurface = surface.wl_surface();

                        draw_surface_tree (
                           renderer,
                           frame,
                           surface,
                           1.0,
                           (0.0, 0.0).into(),
                           &[damage],
                           &log).unwrap();

                        send_frames_surface_tree(surface, start_time.elapsed().as_millis() as u32);
                    }
                });
            })?;

        if let Some(stream) = listener.accept()? {
            println!("Got a client: {:?}", stream);

            let client: Client = display
                .handle()
                .insert_client(stream, Arc::new(ClientState))
                .unwrap();

            clients.push(client);
        }

        display.dispatch_clients(&mut state)?;
        display.flush_clients()?;

        // It is important that all events on the display have been dispatched and flushed to clients before
        // swapping buffers because this operation may block.
        backend.submit(Some(&[damage])).unwrap();
    }
}
