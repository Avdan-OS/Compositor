use std::{
    os::unix::prelude::AsRawFd,
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::Duration,
};

use smithay::{
    backend::renderer::element::{default_primary_scanout_output_compare, RenderElementStates},
    delegate_shm,
    desktop::{
        utils::{
            surface_presentation_feedback_flags_from_states, surface_primary_scanout_output,
            update_surface_primary_scanout_output, OutputPresentationFeedback,
        },
        PopupManager, Space,
    },
    input::{keyboard::XkbConfig, pointer::CursorImageStatus, SeatState},
    output::Output,
    reexports::{
        calloop::{generic::Generic, Interest, LoopHandle, Mode, PostAction},
        wayland_server::Display,
    },
    utils::{Clock, Point, Size},
    wayland::{
        compositor::CompositorState,
        data_device::DataDeviceState,
        fractional_scale::{with_fractional_scale, FractionalScaleManagerState},
        input_method::{InputMethodManagerState, InputMethodSeat},
        keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitState,
        output::OutputManagerState,
        presentation::PresentationState,
        primary_selection::PrimarySelectionState,
        shell::{
            wlr_layer::WlrLayerShellState,
            xdg::{decoration::XdgDecorationState, XdgShellState},
        },
        shm::ShmState,
        socket::ListeningSocketSource,
        tablet_manager::TabletSeatTrait,
        text_input::TextInputManagerState,
        viewporter::ViewporterState,
        virtual_keyboard::VirtualKeyboardManagerState,
        xdg_activation::XdgActivationState,
    },
    xwayland::{X11Wm, XWayland, XWaylandEvent},
};

use crate::compositor::{
    backend::Backend, components::Cursor, shell::AvWindow, state::ClientState,
};

use super::{CalloopData, Navda};

impl<BEnd: Backend + 'static> Navda<BEnd> {
    pub fn new(
        display: &mut Display<Navda<BEnd>>,
        handle: LoopHandle<'static, CalloopData<BEnd>>,
        backend_data: BEnd,
        log: slog::Logger,
    ) -> Self {
        let clock = Clock::new().expect("failed to initialize clock");

        // init wayland clients
        let socket_name = {
            let source = ListeningSocketSource::new_auto(log.clone()).unwrap();
            let socket_name = source.socket_name().to_string_lossy().into_owned();
            handle
                .insert_source(source, |client_stream, _, data| {
                    if let Err(err) = data
                        .display
                        .handle()
                        .insert_client(client_stream, Arc::new(ClientState))
                    {
                        slog::warn!(data.state.log, "Error adding wayland client: {}", err);
                    };
                })
                .expect("Failed to init wayland socket source");
            slog::info!(log, "Listening on wayland socket"; "name" => socket_name.clone());
            ::std::env::set_var("WAYLAND_DISPLAY", &socket_name);
            socket_name
        };

        handle
            .insert_source(
                Generic::new(
                    display.backend().poll_fd().as_raw_fd(),
                    Interest::READ,
                    Mode::Level,
                ),
                |_, _, data| {
                    data.display.dispatch_clients(&mut data.state).unwrap();
                    Ok(PostAction::Continue)
                },
            )
            .expect("Failed to init wayland server source");

        // init globals
        let dh = display.handle();
        let compositor_state = CompositorState::new::<Self, _>(&dh, log.clone());
        let data_device_state = DataDeviceState::new::<Self, _>(&dh, log.clone());
        let layer_shell_state = WlrLayerShellState::new::<Self, _>(&dh, log.clone());
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&dh);
        let primary_selection_state = PrimarySelectionState::new::<Self, _>(&dh, log.clone());
        let mut seat_state = SeatState::new();
        let shm_state = ShmState::new::<Self, _>(&dh, vec![], log.clone());
        let viewporter_state = ViewporterState::new::<Self, _>(&dh, log.clone());
        let xdg_activation_state = XdgActivationState::new::<Self, _>(&dh, log.clone());
        let xdg_decoration_state = XdgDecorationState::new::<Self, _>(&dh, log.clone());
        let xdg_shell_state = XdgShellState::new::<Self, _>(&dh, log.clone());
        let presentation_state = PresentationState::new::<Self>(&dh, clock.id() as u32);
        let fractional_scale_manager_state =
            FractionalScaleManagerState::new::<Self, _>(&dh, log.clone());
        TextInputManagerState::new::<Self>(&dh);
        InputMethodManagerState::new::<Self>(&dh);
        VirtualKeyboardManagerState::new::<Self, _>(&dh, |_client| true);

        // init input
        let seat_name = backend_data.seat_name();
        let mut seat = seat_state.new_wl_seat(&dh, seat_name.clone(), log.clone());

        let cursor_status = Arc::new(Mutex::new(CursorImageStatus::Default));
        seat.add_pointer();
        // TODO: Add keyboard layout support here:
        seat.add_keyboard(XkbConfig::default(), 200, 25)
            .expect("Failed to initialize the keyboard");

        let cursor_status2 = cursor_status.clone();
        seat.tablet_seat()
            .on_cursor_surface(move |_tool, new_status| {
                // TODO: tablet tools should have their own cursors
                *cursor_status2.lock().unwrap() = new_status;
            });

        seat.add_input_method(XkbConfig::default(), 200, 25);

        let dh = display.handle();
        let keyboard_shortcuts_inhibit_state = KeyboardShortcutsInhibitState::new::<Self>(&dh);

        let xwayland = {
            let (xwayland, channel) = XWayland::new(log.clone(), &dh);
            let log2 = log.clone();
            let ret = handle.insert_source(channel, move |event, _, data| match event {
                XWaylandEvent::Ready {
                    connection,
                    client,
                    client_fd: _,
                    display: _,
                } => {
                    let mut wm = X11Wm::start_wm(
                        data.state.handle.clone(),
                        dh.clone(),
                        connection,
                        client,
                        log2.clone(),
                    )
                    .expect("Failed to attach X11 Window Manager");
                    let cursor = Cursor::load(&log2);
                    let image = cursor.get_image(1, Duration::ZERO);
                    wm.set_cursor(
                        &image.pixels_rgba,
                        Size::from((image.width as u16, image.height as u16)),
                        Point::from((image.xhot as u16, image.yhot as u16)),
                    )
                    .expect("Failed to set xwayland default cursor");
                    data.state.xwm = Some(wm);
                }
                XWaylandEvent::Exited => {
                    let _ = data.state.xwm.take();
                }
            });
            if let Err(e) = ret {
                slog::error!(
                    log,
                    "Failed to insert the XWaylandSource into the event loop: {}",
                    e
                );
            }
            xwayland
        };

        Navda {
            backend_data,
            display_handle: display.handle(),
            socket_name,
            running: Arc::new(AtomicBool::new(true)),
            handle,
            space: Space::new(log.clone()),
            popups: PopupManager::new(log.clone()),
            compositor_state,
            data_device_state,
            layer_shell_state,
            output_manager_state,
            primary_selection_state,
            seat_state,
            keyboard_shortcuts_inhibit_state,
            shm_state,
            viewporter_state,
            xdg_activation_state,
            xdg_decoration_state,
            xdg_shell_state,
            presentation_state,
            fractional_scale_manager_state,
            dnd_icon: None,
            log,
            suppressed_keys: Vec::new(),
            pointer_location: (0.0, 0.0).into(),
            cursor_status,
            seat_name,
            seat,
            clock,
            xwayland,
            xwm: None,
        }
    }
}

pub fn post_repaint(
    output: &Output,
    render_element_states: &RenderElementStates,
    space: &Space<AvWindow>,
    time: impl Into<Duration>,
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
            window.send_frame(output, time, throttle, surface_primary_scanout_output);
        }
    });
    let map = smithay::desktop::layer_map_for_output(output);
    for layer_surface in map.layers() {
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
                    fraction_scale.set_preferred_scale(output.current_scale().fractional_scale());
                });
            }
        });

        layer_surface.send_frame(output, time, throttle, surface_primary_scanout_output);
    }
}

pub fn take_presentation_feedback(
    output: &Output,
    space: &Space<AvWindow>,
    render_element_states: &RenderElementStates,
) -> OutputPresentationFeedback {
    let mut output_presentation_feedback = OutputPresentationFeedback::new(output);

    space.elements().for_each(|window| {
        if space.outputs_for_element(window).contains(output) {
            window.take_presentation_feedback(
                &mut output_presentation_feedback,
                surface_primary_scanout_output,
                |surface, _| {
                    surface_presentation_feedback_flags_from_states(surface, render_element_states)
                },
            );
        }
    });
    let map = smithay::desktop::layer_map_for_output(output);
    for layer_surface in map.layers() {
        layer_surface.take_presentation_feedback(
            &mut output_presentation_feedback,
            surface_primary_scanout_output,
            |surface, _| {
                surface_presentation_feedback_flags_from_states(surface, render_element_states)
            },
        );
    }

    output_presentation_feedback
}
