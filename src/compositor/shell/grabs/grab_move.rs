use smithay::{
    input::{
        pointer::{self, GrabStartData as PointerGrabStartData, PointerGrab},
        SeatHandler,
    },
    utils::{Logical, Point},
};

use crate::compositor::{backend::Backend, shell::avwindow::AvWindow, state::Navda};

pub struct MoveSurfaceGrab<B: Backend + 'static> {
    pub start_data: PointerGrabStartData<Navda<B>>,
    pub window: AvWindow,
    pub initial_window_location: Point<i32, Logical>,
}

impl<BEnd: Backend> PointerGrab<Navda<BEnd>> for MoveSurfaceGrab<BEnd> {
    fn motion(
        &mut self,
        data: &mut Navda<BEnd>,
        handle: &mut pointer::PointerInnerHandle<'_, Navda<BEnd>>,
        focus: Option<(
            <Navda<BEnd> as SeatHandler>::PointerFocus,
            Point<i32, Logical>,
        )>,
        event: &pointer::MotionEvent,
    ) {
        // While grab is active, no client has pointer focus.
        handle.motion(data, None, event);

        let delta = event.location - self.start_data.location;
        let new_location = self.initial_window_location.to_f64() + delta;

        data.space
            .map_element(self.window.clone(), new_location.to_i32_round(), true);
    }

    fn button(
        &mut self,
        data: &mut Navda<BEnd>,
        handle: &mut pointer::PointerInnerHandle<'_, Navda<BEnd>>,
        event: &pointer::ButtonEvent,
    ) {
        handle.button(data, event);
        if handle.current_pressed().is_empty() {
            // No more buttons are pressed, release grab.
            handle.unset_grab(data, event.serial, event.time);
        }
    }

    fn axis(
        &mut self,
        data: &mut Navda<BEnd>,
        handle: &mut pointer::PointerInnerHandle<'_, Navda<BEnd>>,
        details: pointer::AxisFrame,
    ) {
        handle.axis(data, details);
    }

    fn start_data(&self) -> &pointer::GrabStartData<Navda<BEnd>> {
        &self.start_data
    }
}
