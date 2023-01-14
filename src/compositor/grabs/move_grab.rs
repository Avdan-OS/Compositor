use smithay::{
    desktop::Window,
    utils::{Logical, Point},
    input::{
        pointer::{
            PointerGrab, PointerInnerHandle, MotionEvent,
            ButtonEvent, AxisFrame, GrabStartData,
            GrabStartData as PointerGrabStartData
        },
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface
};

use crate::compositor::state::Navda;

pub struct MoveSurfaceGrab<BEnd : 'static> {
    pub start_data  : PointerGrabStartData<Navda<BEnd>>,
    pub window      : Window,

    pub initial_window_location : Point<i32, Logical>,
}

impl<BEnd : 'static> PointerGrab<Navda<BEnd>> for MoveSurfaceGrab<BEnd> {
    fn motion(
        &mut self,
        data: &mut Navda<BEnd>,
        handle: &mut PointerInnerHandle<'_, Navda<BEnd>>,
        focus: Option<(WlSurface, Point<i32, Logical>)>,
        event: &MotionEvent,
    ) {
        // While grab active, no client has pointer focus.
        handle.motion(data, None, event);

        let delta = event.location - self.start_data.location;
        let new_location = self.initial_window_location.to_f64() + delta;
        
        // Move this window
        data.space
            .map_element(
            self.window.clone(),
            new_location.to_i32_round(),
            true
        );
    }

    fn button(
        &mut self,
        data: &mut Navda<BEnd>,
        handle: &mut PointerInnerHandle<'_, Navda<BEnd>>,
        event: &ButtonEvent
    ) {

        // For button code:
        // https://github.com/torvalds/linux/blob/master/include/uapi/linux/input-event-codes.h
        const BTN_LEFT: u32 = 0x110;
        
        if !handle.current_pressed().contains(&BTN_LEFT) {
            // If left btn not held down (unpressed), release grab.
            handle.unset_grab(data, event.serial, event.time);
        }
    }

    fn axis(
        &mut self, data:
        &mut Navda<BEnd>,
        handle: &mut PointerInnerHandle<'_, Navda<BEnd>>,
        details: AxisFrame
    ) {
        handle.axis(data, details)    
    }

    fn start_data(&self) -> &GrabStartData<Navda<BEnd>> {
        &self.start_data
    }
}