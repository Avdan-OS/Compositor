use std::{
    ffi::OsString,
    sync::{atomic::Ordering, Mutex},
    time::Duration,
};

use slog::Logger;

use smithay::{
    backend::{
        allocator::dmabuf::Dmabuf,
        renderer::{
            damage::{DamageTrackedRenderer, DamageTrackedRendererError},
            element::AsRenderElements,
            gles2::{Gles2Renderer, Gles2Texture},
            ImportDma, ImportEgl,
        },
        winit::{self, WinitEvent, WinitGraphicsBackend},
        SwapBuffersError,
    },
    delegate_dmabuf,
    input::pointer::{CursorImageAttributes, CursorImageStatus},
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::{
        calloop::EventLoop,
        wayland_protocols::wp::presentation_time::server::wp_presentation_feedback,
        wayland_server::{protocol::wl_surface, Display},
    },
    utils::{IsAlive, Point, Scale, Transform},
    wayland::{
        compositor,
        dmabuf::{DmabufGlobal, DmabufHandler, DmabufState, ImportError},
        input_method::InputMethodSeat,
    },
};

use crate::compositor::{
    drawing::PointerElement,
    render::{render_output, CustomRenderElements},
    shell,
    state::{post_repaint, take_presentation_feedback, CalloopData, Navda},
};

use super::Backend;

pub const OUTPUT_NAME: &str = "winit";

pub struct WinitData {
    backend: WinitGraphicsBackend<Gles2Renderer>,
    damage_tracked_renderer: DamageTrackedRenderer,
    dmabuf_state: Option<(DmabufState, DmabufGlobal)>,
    full_redraw: u8,
}

impl DmabufHandler for Navda<WinitData> {
    fn dmabuf_state(&mut self) -> &mut DmabufState {
        &mut self.backend_data.dmabuf_state.as_mut().unwrap().0
    }

    fn dmabuf_imported(
        &mut self,
        _global: &DmabufGlobal,
        dmabuf: Dmabuf,
    ) -> Result<(), ImportError> {
        self.backend_data
            .backend
            .renderer()
            .import_dmabuf(&dmabuf, None)
            .map(|_| ())
            .map_err(|_| ImportError::Failed)
    }
}
delegate_dmabuf!(Navda<WinitData>);

impl Backend for WinitData {
    fn seat_name(&self) -> String {
        String::from("winit")
    }
    fn reset_buffers(&mut self, _output: &Output) {
        self.full_redraw = 4;
    }
    fn early_import(&mut self, _surface: &wl_surface::WlSurface) {}
}

pub fn run_winit(log: Logger) {
    let mut event_loop = EventLoop::try_new().unwrap();
    let mut display = Display::new().unwrap();

    let (mut backend, mut winit) = match winit::init::<Gles2Renderer, _>(log.clone()) {
        Ok(ret) => ret,
        Err(err) => {
            slog::crit!(log, "Failed to initialize Winit backend: {}", err);
            return;
        }
    };
    let size = backend.window_size().physical_size;

    let mode = Mode {
        size,
        refresh: 60_000,
    };
    let output = Output::new(
        OUTPUT_NAME.to_string(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "Smithay".into(),
            model: "Winit".into(),
        },
        log.clone(),
    );
    let _global = output.create_global::<Navda<WinitData>>(&display.handle());
    output.change_current_state(
        Some(mode),
        Some(Transform::Flipped180),
        None,
        Some((0, 0).into()),
    );
    output.set_preferred(mode);

    let data = {
        let dmabuf_state = if backend
            .renderer()
            .bind_wl_display(&display.handle())
            .is_ok()
        {
            slog::info!(log, "EGL hardware-acceleration enabled");
            let dmabuf_formats = backend
                .renderer()
                .dmabuf_formats()
                .cloned()
                .collect::<Vec<_>>();
            let mut state = DmabufState::new();
            let global = state.create_global::<Navda<WinitData>, _>(
                &display.handle(),
                dmabuf_formats,
                log.clone(),
            );
            Some((state, global))
        } else {
            None
        };

        let damage_tracked_renderer = DamageTrackedRenderer::from_output(&output);

        WinitData {
            backend,
            damage_tracked_renderer,
            dmabuf_state,
            full_redraw: 0,
        }
    };
    let mut state = Navda::new(&mut display, event_loop.handle(), data, log.clone());
    state.space.map_output(&output, (0, 0));

    if let Err(e) = state.xwayland.start(
        state.handle.clone(),
        None,
        std::iter::empty::<(OsString, OsString)>(),
        |_| {},
    ) {
        slog::error!(log, "Failed to start XWayland: {}", e);
    }

    slog::info!(log, "Initialization completed, starting the main loop.");

    let mut pointer_element = PointerElement::<Gles2Texture>::default();

    while state.running.load(Ordering::SeqCst) {
        if winit
            .dispatch_new_events(|event| match event {
                WinitEvent::Resized { size, .. } => {
                    // We only have one output
                    let output = state.space.outputs().next().unwrap().clone();
                    state.space.map_output(&output, (0, 0));
                    let mode = Mode {
                        size,
                        refresh: 60_000,
                    };
                    output.change_current_state(Some(mode), None, None, None);
                    output.set_preferred(mode);
                    shell::fixup_positions(&mut state.space);
                }
                WinitEvent::Input(event) => {
                    state.process_input_event_windowed(&display.handle(), event, OUTPUT_NAME)
                }
                _ => (),
            })
            .is_err()
        {
            state.running.store(false, Ordering::SeqCst);
            break;
        }

        // drawing logic
        {
            let backend = &mut state.backend_data.backend;

            let mut cursor_guard = state.cursor_status.lock().unwrap();

            // draw the cursor as relevant
            // reset the cursor if the surface is no longer alive
            let mut reset = false;
            if let CursorImageStatus::Surface(ref surface) = *cursor_guard {
                reset = !surface.alive();
            }
            if reset {
                *cursor_guard = CursorImageStatus::Default;
            }
            let cursor_visible = !matches!(*cursor_guard, CursorImageStatus::Surface(_));

            pointer_element.set_status(cursor_guard.clone());

            let full_redraw = &mut state.backend_data.full_redraw;
            *full_redraw = full_redraw.saturating_sub(1);
            let space = &mut state.space;
            let damage_tracked_renderer = &mut state.backend_data.damage_tracked_renderer;

            let input_method = state.seat.input_method().unwrap();
            let dnd_icon = state.dnd_icon.as_ref();

            let scale = Scale::from(output.current_scale().fractional_scale());
            let cursor_hotspot = if let CursorImageStatus::Surface(ref surface) = *cursor_guard {
                compositor::with_states(surface, |states| {
                    states
                        .data_map
                        .get::<Mutex<CursorImageAttributes>>()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .hotspot
                })
            } else {
                (0, 0).into()
            };
            let cursor_pos = state.pointer_location - cursor_hotspot.to_f64();
            let cursor_pos_scaled = cursor_pos.to_physical(scale).to_i32_round();

            let render_res = backend.bind().and_then(|_| {
                let age = if *full_redraw > 0 {
                    0
                } else {
                    backend.buffer_age().unwrap_or(0)
                };

                let renderer = backend.renderer();

                let mut elements = Vec::<CustomRenderElements<Gles2Renderer>>::new();

                elements.extend(pointer_element.render_elements(
                    renderer,
                    cursor_pos_scaled,
                    scale,
                ));

                // draw input method surface if any
                let rectangle = input_method.coordinates();
                let position = Point::from((
                    rectangle.loc.x + rectangle.size.w,
                    rectangle.loc.y + rectangle.size.h,
                ));
                input_method.with_surface(|surface| {
                    elements.extend(AsRenderElements::<Gles2Renderer>::render_elements(
                        &smithay::desktop::space::SurfaceTree::from_surface(surface),
                        renderer,
                        position.to_physical_precise_round(scale),
                        scale,
                    ));
                });

                // draw the dnd icon if any
                if let Some(surface) = dnd_icon {
                    if surface.alive() {
                        elements.extend(AsRenderElements::<Gles2Renderer>::render_elements(
                            &smithay::desktop::space::SurfaceTree::from_surface(surface),
                            renderer,
                            cursor_pos_scaled,
                            scale,
                        ));
                    }
                }

                render_output(
                    &output,
                    space,
                    &elements,
                    renderer,
                    damage_tracked_renderer,
                    age,
                    &log,
                )
                .map_err(|err| match err {
                    DamageTrackedRendererError::Rendering(err) => err.into(),
                    _ => unreachable!(),
                })
            });

            match render_res {
                Ok((damage, states)) => {
                    let has_rendered = damage.is_some();
                    if let Some(damage) = damage {
                        if let Err(err) = backend.submit(Some(&*damage)) {
                            slog::warn!(log, "Failed to submit buffer: {}", err);
                        }
                    }
                    backend.window().set_cursor_visible(cursor_visible);

                    // Send frame events so that client start drawing their next frame
                    let time = state.clock.now();
                    post_repaint(&output, &states, &state.space, time);

                    if has_rendered {
                        let mut output_presentation_feedback =
                            take_presentation_feedback(&output, &state.space, &states);
                        output_presentation_feedback.presented(
                            time,
                            output
                                .current_mode()
                                .map(|mode| mode.refresh as u32)
                                .unwrap_or_default(),
                            0,
                            wp_presentation_feedback::Kind::Vsync,
                        )
                    }
                }
                Err(SwapBuffersError::ContextLost(err)) => {
                    slog::error!(log, "Critical Rendering Error: {}", err);
                    state.running.store(false, Ordering::SeqCst);
                }
                Err(err) => slog::warn!(log, "Rendering error: {}", err),
            }
        }

        let mut calloop_data = CalloopData { state, display };
        let result = event_loop.dispatch(Some(Duration::from_millis(1)), &mut calloop_data);
        CalloopData { state, display } = calloop_data;

        if result.is_err() {
            state.running.store(false, Ordering::SeqCst);
        } else {
            state.space.refresh();
            state.popups.cleanup();
            display.flush_clients().unwrap();
        }
    }
}
