use std::{
    result::Result,
    sync::atomic::Ordering,
    marker::PhantomData, time::Duration,
};

use smithay::{
    backend::{
        winit::{self, WinitGraphicsBackend, WinitEventLoop, Error, WinitEvent, WinitError, WindowSize},
        renderer::{
            gles2::{Gles2Renderer, Gles2Texture},
            damage::{DamageTrackedRenderer, DamageTrackedRendererError}, element::{AsRenderElements, RenderElementStates, surface::WaylandSurfaceRenderElement}, Renderer
        }, self, SwapBuffersError
    }, 
    output::{
        Output, PhysicalProperties, Mode, Subpixel
    },
    reexports::{
        wayland_server::{
            protocol::wl_surface::WlSurface, Display},
            calloop::EventLoop, wayland_protocols::wp::presentation_time::server::wp_presentation_feedback,
        }, utils::{Transform, Physical, Size, Scale, Point, Rectangle}, wayland::input_method::{InputMethodHandle, InputMethodSeat}, desktop::{space::{SurfaceTree, render_output}, Window, Space}
    };

use crate::compositor::{state::{Navda, post_repaint, take_presentation_feedback}, CalloopData};

use super::{Backend, NavdaBackend};

pub const OUTPUT_NAME: &str = "winit";

pub struct WinitData {
    backend: WinitGraphicsBackend<Gles2Renderer>,
    damage_tracked_renderer: DamageTrackedRenderer,
    full_redraw: u8,
}

impl Backend for WinitData {
    fn seat_name(&self) -> String {
        "winit".to_string()
    }

    fn reset_buffers(
        &mut self,
        output: &Output
    ) {
        self.full_redraw = 4;    
    }

    fn early_import(
        &mut self,
        surface: &WlSurface
    ) {}
}

struct CustomRenderElements<R : Renderer>(
    PhantomData<R>
);

/// 
/// Helper struct for implementing the 
/// `winit` backend.
/// 
pub struct Winit;

impl NavdaBackend for Winit {
    type Data = WinitData;
    fn run(
        log : slog::Logger
    ) -> Result<(), Box<dyn std::error::Error>> {
        
        let mut event_loop = EventLoop::try_new().unwrap();
        let mut display = Display::new().unwrap();
        
        let (backend, mut winit) = Winit::winit_backend(log.clone())?;

        let output = Self::create_output(
            &display, backend.window_size().physical_size, &log
        );

        let data = {
            let damage_tracked_renderer = DamageTrackedRenderer::from_output(&output);

            WinitData {
                backend,
                damage_tracked_renderer,
                full_redraw: 0,
            }
        };

        let mut state = Navda::new(
            &mut display,
            event_loop.handle(),
            data,
            log.clone()
        );

        state.space.map_output(&output, (0,0));

        // TODO: XWayland Support @ anvil/winit.rs:163

        slog::info!(
            log, "Initialization completed, starting main loop."
        );

        // let mut pointer_element = PointerElement::<Gles2Texture>::default();

        // Main loop here...
        while state.running.load(Ordering::SeqCst) {
            if let Err(err) = Winit::dispatch_events(&mut state, &display, &mut winit)
            {
                Winit::stop_compositor(&mut state);
                return Err(err.into());
            }

            Self::draw(&mut state, &output);
            (event_loop, state, display) = Self::post_draw(event_loop, state, display);
        }

        Ok(())
    }
}

impl Winit {
    fn winit_backend(
        log : slog::Logger
    ) -> Result<
        (WinitGraphicsBackend<Gles2Renderer>, WinitEventLoop), Error> {
        winit::init::<Gles2Renderer, _>(log.clone())
            .map_err(|err| {
                slog::crit!(log, "Failed to initialize Winit backend: {}", err);
                err
            })
    }

    fn create_output(
        display : &Display<Navda<WinitData>>,
        size    : Size<i32, Physical>,
        log     : &slog::Logger
    ) -> Output {

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
            log.clone()
        );

        output.create_global::<Navda<WinitData>>(&display.handle());
        output.change_current_state(Some(mode), Some(Transform::Flipped180), None, Some((0, 0).into()));
        output.set_preferred(mode);

        output
    }

    fn dispatch_events(
        state   : &mut Navda<WinitData>,
        display : &Display<Navda<WinitData>>,
        winit   : &mut WinitEventLoop,
    ) -> Result<(), WinitError> {
        winit
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
                    // crate::shell::fixup_positions(&mut state.space);
                }
                WinitEvent::Input(event) => {
                    state.process_input_event_windowed(&display.handle(), event, OUTPUT_NAME)
                }
                _ => (),
            })
    }

    fn render<'a>(
        output      : &'a Output,
        state       : &'a mut Navda<WinitData>,
        scale       : &'a Scale<f64>, 
    ) -> impl FnOnce(()) -> Result<
            (Option<Vec<Rectangle<i32, Physical>>>, RenderElementStates), 
            SwapBuffersError
        > + 'a
    {
        move |_| {
            let backend = &mut state.backend_data.backend;
            let full_redraw = &mut state.backend_data.full_redraw;
            // let input_method = state.seat.input_method().unwrap();


            let age = if *full_redraw > 0 {
                0
            } else {
                backend.buffer_age().unwrap_or(0)
            };
    
            let renderer = backend.renderer();
    
            // TODO: Cursor and inputs Below,
            let elements = Vec::<WaylandSurfaceRenderElement<Gles2Renderer>>::new();
    
            // elements.extend(pointer_element.render_elements(renderer, cursor_pos_scaled, scale));
    
            // draw input method surface if any
     
            // TODO: Input Method stuff @ anvil/winit.rs:{264-271}

            // TODO: Drag and drop icon @ anvil/winit.rs:{274-283}


            render_output(
                output,
                renderer,
                age,
                [&state.space],
                &elements,
                &mut state.backend_data.damage_tracked_renderer,
                [0.1, 0.1, 0.1, 1.0],
                state.log.clone()
            ).map_err(|err| match err {
                DamageTrackedRendererError::Rendering(e) => e.into(),
                _ => unreachable!(),
            })
        }
    }

    fn post_render(
        state           : &mut Navda<WinitData>,
        output          : &Output,
        render_output   : (Option<Vec<Rectangle<i32, Physical>>>, RenderElementStates)
    ) -> () {
        let (damage, states) = render_output;
        let backend = &mut state.backend_data.backend;

        let has_rendered = damage.is_some();
        if let Some(damage) = damage {
            if let Err(err) = backend.submit(Some(&*damage)) {
                slog::warn!(state.log, "Failed to submit buffer: {}", err);
            }
        }
        // backend.window().set_cursor_visible(cursor_visible);

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

    fn draw(
        state   : &mut Navda<WinitData>,
        output  : &Output,
    ) -> () {

        // TODO: Cursor things @ anvil/winit.rs:{199-212}
        
        {
            let full_redraw = &mut state.backend_data.full_redraw;
            *full_redraw = full_redraw.saturating_sub(1);
        }


        // let dnd_icon = state.dnd_icon.as_ref();

        let scale = Scale::from(output.current_scale().fractional_scale());

        // TODO: Cursor things @ anvil/winit.rs:{229-243}

        // Render things
        let render_res = state.backend_data.backend.bind();
        let e = render_res.and_then(
            Self::render(
                output, state,
                &scale
            )
        );

        match e {
            Ok(render_output) => Self::post_render(state, output, render_output),
            Err(SwapBuffersError::ContextLost(err)) => {
                slog::error!(state.log, "Critical Rendering Error: {}", err);
                Winit::stop_compositor(state);
            }
            Err(err) => slog::warn!(state.log, "Rendering error: {}", err),
        }
        
    }

    fn post_draw(
        mut event_loop  : EventLoop<CalloopData<WinitData>>,
        mut state       : Navda<WinitData>,
        mut display     : Display<Navda<WinitData>> 
    ) -> (EventLoop<CalloopData<WinitData>>, Navda<WinitData>, Display<Navda<WinitData>>) {
        let mut calloop_data = CalloopData { state, display };
        let result = event_loop.dispatch(Some(Duration::from_millis(1)), &mut calloop_data);
        CalloopData { state, display } = calloop_data;

        if result.is_err() {
            state.running.store(false, Ordering::SeqCst);
        } else {
            state.space.refresh();
            // TODO: Popups
            // state.popups.cleanup();
            display.flush_clients().unwrap();
        }
        (event_loop, state, display)
    }
}