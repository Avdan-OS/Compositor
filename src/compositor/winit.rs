use crate::{
    AvCompositor,
    CalloopData,
};

use smithay::{
    backend::{
        renderer::gles2::Gles2Renderer,
        winit::{
            self,
            WinitError,
            WinitEvent,
            WinitEventLoop,
            WinitGraphicsBackend,
        },
    },
    desktop::space::SurfaceTree,
    output::{
        Mode,
        Output,
        PhysicalProperties,
        Subpixel,
    },
    reexports::{
        calloop::{
            timer::{
                TimeoutAction, 
                Timer,
            },
            EventLoop,
        }, 
        wayland_server::{
            backend::GlobalId,
            Display,
        },
    },
    utils::{
        Physical,
        Rectangle,
        Size,
        Transform,
    },
};

use slog::Logger;

use std::time::Duration;

pub fn init_winit (
    event_loop: &mut EventLoop<CalloopData>,
    data      : &mut CalloopData,
    log       : Logger,
) -> Result<(), Box<dyn std::error::Error>> {
    let display: &mut Display<AvCompositor> = &mut data.display; //: &mut Display<AvCompositor>
    let state: &mut AvCompositor = &mut data.state;

    let (mut backend, mut winit): (WinitGraphicsBackend, WinitEventLoop) = winit::init(log.clone())?;

    let mode: Mode = Mode {
        size: backend.window_size().physical_size,
        refresh: 60_000,
    };

    let output: Output = Output::new::<_> (
        "winit".to_string(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "AvdanOS Wayland Compositor".into(),
            model: "Winit".into(),
        },
        log.clone(),
    );

    let _global: GlobalId = output.create_global::<AvCompositor>(&display.handle());
    output.change_current_state(Some(mode), Some(Transform::Flipped180), None, Some((0, 0).into()));
    output.set_preferred(mode);

    state.space.map_output(&output, (0, 0));

    std::env::set_var("WAYLAND_DISPLAY", &state.socket_name);

    let mut full_redraw: u8 = 0u8;

    let timer: Timer = Timer::immediate();
    event_loop.handle().insert_source(timer, move |_, _, data: &mut CalloopData| {
        winit_dispatch(&mut backend, &mut winit, data, &output, &mut full_redraw).unwrap();
        TimeoutAction::ToDuration(Duration::from_millis(16))
    })?;

    Ok(())
}

pub fn winit_dispatch(
    backend    : &mut WinitGraphicsBackend,
    winit      : &mut WinitEventLoop,
    data       : &mut CalloopData,
    output     : &Output,
    full_redraw: &mut u8,
) -> Result<(), Box<dyn std::error::Error>> {
    let display: &mut Display<AvCompositor> = &mut data.display;
    let state  : &mut AvCompositor          = &mut data.state;

    let res: Result<(), WinitError> = winit.dispatch_new_events(|event: WinitEvent|
        match event {
            WinitEvent::Resized { size, .. } => {
                output.change_current_state (
                    Some(Mode {
                        size,
                        refresh: 60_000,
                    }),
                    None,
                    None,
                    None,
                );
            },

            WinitEvent::Input(event) => state.process_input_event(event),

            _ => (),
        }
    );

    if let Err(WinitError::WindowClosed) = res {
        // Stop the loop
        state.loop_signal.stop();

        return Ok(());
    } else { res? }

    *full_redraw = full_redraw.saturating_sub(1);

    let size  : Size<i32, Physical>      = backend.window_size().physical_size;
    let damage: Rectangle<i32, Physical> = Rectangle::from_loc_and_size((0, 0), size);

    backend.bind().ok().and_then(|_| {
        state
            .space
            .render_output::<Gles2Renderer, SurfaceTree>(
                backend.renderer(),
                output,
                0,
                [0.1, 0.1, 0.1, 1.0],
                &[],
            )
            .unwrap()
    });

    backend.submit(Some(&[damage])).unwrap();

    state
        .space
        .send_frames(state.start_time.elapsed().as_millis() as u32);

    state.space.refresh(&display.handle());
    display.flush_clients()?;

    Ok(())
}
