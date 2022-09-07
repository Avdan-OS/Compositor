#![allow(unused_parens)]

use smithay::{
    backend::input::{
        AbsolutePositionEvent,
        Axis,
        AxisSource,
        ButtonState,
        Event,
        InputBackend,
        InputEvent,
        KeyboardKeyEvent, 
        PointerAxisEvent,
        PointerButtonEvent,
    },
    input::{
        keyboard::{FilterResult, KeyboardHandle},
        pointer::{
            AxisFrame,
            ButtonEvent,
            MotionEvent,
            PointerHandle,
        },
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{
        Logical,
        Point,
        Rectangle,
        Serial,
        SERIAL_COUNTER,
    },
};

use crate::AvCompositor;

impl AvCompositor {
    pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
        match event {
            InputEvent::Keyboard { event, .. } => {
                let serial: Serial = SERIAL_COUNTER.next_serial();
                let time  : u32    = Event::time(&event);

                self.seat.get_keyboard().unwrap().input::<(), _> (
                    self,
                    event.key_code(),
                    event.state(),
                    serial,
                    time,
                    |_, _, _| FilterResult::Forward,
                );
            },

            InputEvent::PointerMotion { .. } => {},
            
            InputEvent::PointerMotionAbsolute { event, .. } => {
                let output = self.space.outputs().next().unwrap(); //: &Output

                let output_geo: Rectangle<i32, Logical> = self.space.output_geometry(output).unwrap();

                let pos: Point<f64, Logical> = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();

                let serial: Serial = SERIAL_COUNTER.next_serial();

                let pointer: PointerHandle<AvCompositor> = self.seat.get_pointer().unwrap();

                let under = self.surface_under_pointer(&pointer); //: Option<(WlSurface, Point<f64, _>)>

                pointer.motion (
                    self,
                    under,
                    &MotionEvent {
                        location: pos,
                        serial,
                        time: event.time(),
                    },
                );
            },

            InputEvent::PointerButton { event, .. } => {
                let pointer : PointerHandle<AvCompositor> = self.seat.get_pointer().unwrap();
                let keyboard: KeyboardHandle<AvCompositor> = self.seat.get_keyboard().unwrap();

                let serial: Serial = SERIAL_COUNTER.next_serial();

                let button: u32 = event.button_code();

                let button_state: ButtonState = event.state();

                if (ButtonState::Pressed == button_state && !pointer.is_grabbed()) {
                    if let Some(window) = self.space.window_under(pointer.current_location()).cloned() {
                        self.space.raise_window(&window, true);
                        keyboard.set_focus(self, Some(window.toplevel().wl_surface().clone()), serial);
                        window.set_activated(true);
                        window.configure();
                    } else {
                        self.space.windows().for_each(|window| { //: &Window
                            window.set_activated(false);
                            window.configure();
                        });
                        
                        keyboard.set_focus(self, Option::<WlSurface>::None, serial);
                    }
                };

                pointer.button (
                    self,
                    &ButtonEvent {
                        button,
                        state: button_state,
                        serial,
                        time: event.time(),
                    },
                );
            },

            InputEvent::PointerAxis { event, .. } => {
                let source: AxisSource = event.source();

                let horizontal_amount: f64 = event
                    .amount(Axis::Horizontal)
                    .unwrap_or_else(|| event.amount_discrete(Axis::Horizontal).unwrap() * 3.0);
                let vertical_amount  : f64 = event
                    .amount(Axis::Vertical)
                    .unwrap_or_else(|| event.amount_discrete(Axis::Vertical).unwrap() * 3.0);
                
                let horizontal_amount_discrete: Option<f64> = event.amount_discrete(Axis::Horizontal);
                let vertical_amount_discrete  : Option<f64> = event.amount_discrete(Axis::Vertical);

                let mut frame: AxisFrame = AxisFrame::new(event.time()).source(source);

                if (horizontal_amount != 0.0) {
                    frame = frame.value(Axis::Horizontal, horizontal_amount);

                    if let Some(discrete) = horizontal_amount_discrete {
                        frame = frame.discrete(Axis::Horizontal, discrete as i32);
                    }
                } else if (source == AxisSource::Finger) {
                    frame = frame.stop(Axis::Horizontal);
                }

                if (vertical_amount != 0.0) {
                    frame = frame.value(Axis::Vertical, vertical_amount);
                    
                    if let Some(discrete) = vertical_amount_discrete {
                        frame = frame.discrete(Axis::Vertical, discrete as i32);
                    }
                } else if (source == AxisSource::Finger) {
                    frame = frame.stop(Axis::Vertical);
                }

                self.seat.get_pointer().unwrap().axis(self, frame);
            },

            _ => {}
        }
    }
}
