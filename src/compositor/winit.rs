use std::time::Duration;

use slog::Logger;
use smithay::{reexports::{calloop::{EventLoop, timer::{Timer, TimeoutAction}}, }, backend::{winit::{self, WinitGraphicsBackend, WinitEventLoop, WinitEvent, WinitError}, renderer::{damage::DamageTrackedRenderer, gles2::Gles2Renderer, element::surface::WaylandSurfaceRenderElement}}, output::{Mode, Output, PhysicalProperties, Subpixel}, utils::{Transform, Rectangle}, desktop::space::render_output};

use super::{CalloopData, state::Navda};

pub fn init_winit(
    event_loop  : &mut EventLoop<CalloopData>,
    data        : &mut CalloopData,
    log         : Logger,
) -> Result<(), Box<dyn std::error::Error>> {
    let display = &mut data.display;
    let state = &mut data.state;

    let (mut backend, mut winit) = winit::init(log.clone())?;

    // Try to go at a solid 60Hz.
    let mode = Mode {
        size    : backend.window_size().physical_size,
        refresh : 60_000,
    };

    let output = Output::new::<_>(
        "winit".to_string(),
        PhysicalProperties {
            size    : (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make    : "Navda".into(),
            model   : "Winit".into(),
        },
        log.clone()
    );

    output.create_global::<Navda>(&display.handle());
    output.change_current_state(
        Some(mode),
        Some(Transform::Flipped180),
        None,
        Some((0,0).into())
    );
    output.set_preferred(mode);

    state.space.map_output(&output, (0, 0));

    let mut damage_tracked_render = DamageTrackedRenderer::from_output(
        &output
    );

    std::env::set_var("WAYLAND_DISPLAY", &state.socket_name);

    let mut full_redraw = 0u8;

    let timer = Timer::immediate();
    event_loop.handle().insert_source(timer, move |_, _, data| {
        winit_dispatch(
            &mut backend,
            &mut winit,
            data,
            &output,
            &mut damage_tracked_render,
            &mut full_redraw,
            &log
        )
        .unwrap();

        // Set timeout to ~16ms or ~1 'frame'.
        TimeoutAction::ToDuration(Duration::from_millis(16))
    })?;

    Ok(())
}

pub fn winit_dispatch(
    backend     : &mut WinitGraphicsBackend<Gles2Renderer>,
    winit       : &mut WinitEventLoop,
    data        : &mut CalloopData,
    output      : &Output,
    
    damage_tracked_renderer : &mut DamageTrackedRenderer,
    full_redraw : &mut u8,
    log         : &Logger
) -> Result<(), Box<dyn std::error::Error>> {
    let display = &mut data.display;
    let state = &mut data.state;

    let res = winit.dispatch_new_events(|event| match event {
        WinitEvent::Resized { size, .. } => {
            output.change_current_state(
                Some(Mode { size, refresh: 60_000, }),
                None,
                 None,
                None
            );
        },
        WinitEvent::Input(event) => state.process_input_event(event),
        _ => (),
    });

    if let Err(WinitError::WindowClosed) = res {
        state.loop_signal.stop();
        return Ok(());
    } else {
        res?;
    }

    *full_redraw = full_redraw.saturating_sub(1);

    let size = backend.window_size().physical_size;
    let damage = Rectangle::from_loc_and_size((0, 0), size);

    backend.bind()?;
    render_output::<
        _, WaylandSurfaceRenderElement<Gles2Renderer>,
        _, _, _>
    (
        output,
        backend.renderer(), 
        0,
        [&state.space],
        &[],
        damage_tracked_renderer,
        [0.1, 0.1, 0.1, 1.0],
        log.clone(),
    )?;
    backend.submit(Some(&[damage]))?;

    state.space.elements().for_each(|window| {
        window.send_frame(
            output,
            state.start_time.elapsed(),
            Some(Duration::ZERO),
            |_, _| Some(output.clone()),
        )
    });
    
    state.space.refresh();
    display.flush_clients()?;

    Ok(())
}