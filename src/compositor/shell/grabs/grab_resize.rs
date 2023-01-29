//!
//! Grab handlers for resizing windows.
//!

use std::cell::RefCell;

use smithay::{
    desktop::space::SpaceElement,
    input::{
        pointer::{self, GrabStartData as PointerGrabStartData, PointerGrab},
        SeatHandler,
    },
    reexports::wayland_protocols::xdg::shell::server::xdg_toplevel,
    utils::{IsAlive, Logical, Point, Rectangle, Serial, Size},
    wayland::{
        compositor::{self},
        shell::xdg::SurfaceCachedState,
    },
    xwayland,
};

use crate::compositor::{
    backend::Backend,
    shell::{avwindow::AvWindow, SurfaceData},
    state::Navda,
};

bitflags::bitflags! {
    ///
    /// Edge/Corner that was grabbed, if any.
    ///
    /// Here, we are using the `smallvil`'s implementation,
    /// + NONE.
    ///
    ///
    pub struct ResizeEdge: u32 {
        const NONE          = 0b0000;

        const TOP           = 0b0001;
        const BOTTOM        = 0b0010;
        const LEFT          = 0b0100;
        const RIGHT         = 0b1000;

        const TOP_LEFT      = Self::TOP.bits    | Self::LEFT.bits;
        const BOTTOM_LEFT   = Self::BOTTOM.bits | Self::LEFT.bits;
        const TOP_RIGHT     = Self::TOP.bits    | Self::RIGHT.bits;
        const BOTTOM_RIGHT  = Self::BOTTOM.bits | Self::RIGHT.bits;
    }
}

impl From<xdg_toplevel::ResizeEdge> for ResizeEdge {
    #[inline]
    fn from(x: xdg_toplevel::ResizeEdge) -> Self {
        Self::from_bits(x as u32).unwrap()
    }
}

impl From<ResizeEdge> for xdg_toplevel::ResizeEdge {
    #[inline]
    fn from(x: ResizeEdge) -> Self {
        Self::try_from(x.bits()).unwrap()
    }
}

impl From<xwayland::xwm::ResizeEdge> for ResizeEdge {
    fn from(edge: xwayland::xwm::ResizeEdge) -> Self {
        use xwayland::xwm::ResizeEdge::*;

        match edge {
            Bottom => Self::BOTTOM,
            BottomLeft => Self::BOTTOM_LEFT,
            BottomRight => Self::BOTTOM_RIGHT,
            Left => Self::LEFT,
            Top => Self::TOP,
            TopLeft => Self::TOP_LEFT,
            Right => Self::RIGHT,
            TopRight => Self::TOP_RIGHT,
        }
    }
}

//
/// Data associated with resize grabbing
///
pub struct ResizeSurfaceGrab<BEnd: Backend + 'static> {
    pub start_data: PointerGrabStartData<Navda<BEnd>>,
    pub window: AvWindow,

    pub edges: ResizeEdge,

    pub initial_rect: Rectangle<i32, Logical>,
    pub last_window_size: Size<i32, Logical>,
}

impl<BEnd: Backend> PointerGrab<Navda<BEnd>> for ResizeSurfaceGrab<BEnd> {
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
        // While grab active, no client has focus.
        handle.motion(data, None, event);

        // If dead toplevel, we can't get `min_size` or `max_size`
        // so return early
        if !self.window.alive() {
            handle.unset_grab(data, event.serial, event.time);
            return;
        }

        let mut delta = event.location - self.start_data.location;

        let Size {
            w: mut new_window_width,
            h: mut new_window_height,
            ..
        } = self.initial_rect.size;

        if self.edges.intersects(ResizeEdge::LEFT | ResizeEdge::RIGHT) {
            if self.edges.intersects(ResizeEdge::LEFT) {
                delta.x = -delta.x;
            }

            new_window_width = (self.initial_rect.size.w as f64 + delta.x) as i32;
        }

        if self.edges.intersects(ResizeEdge::TOP | ResizeEdge::BOTTOM) {
            if self.edges.intersects(ResizeEdge::TOP) {
                delta.y = -delta.y;
            }

            new_window_height = (self.initial_rect.size.h as f64 + delta.y) as i32;
        }

        let (min_size, max_size) = self
            .window
            .wl_surface()
            .map(|ref s| {
                compositor::with_states(s, |states| {
                    let data = states.cached_state.current::<SurfaceCachedState>();
                    (data.min_size, data.max_size)
                })
            })
            .unwrap_or(((0, 0).into(), (0, 0).into()));

        let (min_width, min_height) = (min_size.w.max(1), min_size.h.max(1));
        let (max_width, max_height) = (
            (max_size.w == 0).then(|| i32::MAX).unwrap_or(max_size.w),
            (max_size.h == 0).then(|| i32::MAX).unwrap_or(max_size.h),
        );

        self.last_window_size = ((
            new_window_width.max(min_width).min(max_width),
            new_window_height.max(min_height).min(max_height),
        ))
            .into();

        match &self.window {
            AvWindow::Wayland(w) => {
                let xdg = w.toplevel();
                xdg.with_pending_state(|state| {
                    state.states.set(xdg_toplevel::State::Resizing);
                    state.size = Some(self.last_window_size);
                });
                xdg.send_configure();
            }
            AvWindow::X11(x11) => {
                let location = data.space.element_location(&self.window).unwrap();
                x11.configure(Rectangle::from_loc_and_size(
                    location,
                    self.last_window_size,
                ))
                .unwrap();
            }
        }
    }

    fn button(
        &mut self,
        data: &mut Navda<BEnd>,
        handle: &mut pointer::PointerInnerHandle<'_, Navda<BEnd>>,
        event: &pointer::ButtonEvent,
    ) {
        handle.button(data, event);

        /// The button is a button code as defined in the
        /// Linux kernel's [linux/input-event-codes.h](https://github.com/torvalds/linux/blob/master/include/uapi/linux/input-event-codes.h) header file, e.g. BTN_LEFT.
        const BTN_LEFT: u32 = 0x110;

        if !handle.current_pressed().contains(&BTN_LEFT) {
            // No more buttons pressed, release grab.
            handle.unset_grab(data, event.serial, event.time);

            // If toplevel is dead, we can't resize it, so we return early.
            if !self.window.alive() {
                return;
            }

            match &self.window {
                AvWindow::Wayland(w) => {
                    let xdg = w.toplevel();
                    xdg.with_pending_state(|state| {
                        state.states.unset(xdg_toplevel::State::Resizing);
                        state.size = Some(self.last_window_size);
                    });

                    xdg.send_configure();

                    grab_edge_offset_loc(&mut self.edges, data, &self.window, self.initial_rect);

                    compositor::with_states(&self.window.wl_surface().unwrap(), |states| {
                        let mut data = states
                            .data_map
                            .get::<RefCell<SurfaceData>>()
                            .unwrap()
                            .borrow_mut();

                        if let ResizeState::Resizing(resize_data) = data.resize_state {
                            data.resize_state =
                                ResizeState::WaitingForFinalAck(resize_data, event.serial);
                        } else {
                            panic!("invalid resize state: {:?}", data.resize_state);
                        }
                    });
                }
                AvWindow::X11(x11) => {
                    let loc = grab_edge_offset_loc(
                        &mut self.edges,
                        data,
                        &self.window,
                        self.initial_rect,
                    );

                    x11.configure(Rectangle::from_loc_and_size(loc, self.last_window_size))
                        .unwrap();

                    let Some(surface) = self.window.wl_surface() else {
                        // X11 Window got unmapped, abort
                        return
                    };

                    compositor::with_states(&surface, |states| {
                        let mut data = states
                            .data_map
                            .get::<RefCell<SurfaceData>>()
                            .unwrap()
                            .borrow_mut();

                        if let ResizeState::Resizing(resize_data) = data.resize_state {
                            data.resize_state = ResizeState::WaitingForCommit(resize_data);
                        } else {
                            panic!("invalid resize state: {:?}", data.resize_state);
                        }
                    });
                }
            }
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

fn grab_edge_offset_loc<BEnd: 'static + Backend>(
    edges: &mut ResizeEdge,
    data: &mut Navda<BEnd>,
    window: &AvWindow,
    initial_window_rect: Rectangle<i32, Logical>,
) -> Point<i32, Logical> {
    let mut location = data.space.element_location(window).unwrap();
    let initial_window_location = initial_window_rect.loc;
    let initial_window_size = initial_window_rect.size;
    if edges.intersects(ResizeEdge::TOP_LEFT) {
        let geometry = window.geometry();

        if edges.intersects(ResizeEdge::LEFT) {
            location.x = initial_window_location.x + (initial_window_size.w - geometry.size.w);
        }
        if edges.intersects(ResizeEdge::TOP) {
            location.y = initial_window_location.y + (initial_window_size.h - geometry.size.h);
        }

        data.space.map_element(window.clone(), location, true);

        location
    } else {
        location
    }
}

/// Information about the resize operation.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct ResizeData {
    /// The edges the surface is being resized with.
    pub edges: ResizeEdge,

    /// The initial window rectangle.
    pub initial_rect: Rectangle<i32, Logical>,
}

///
/// State of operations.
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeState {
    /// The surface is not being resized.
    Idle,
    /// The surface is currently being resized.
    Resizing(ResizeData),
    /// The resize has finished, and the surface needs to ack the final configure.
    WaitingForFinalAck(ResizeData, Serial),
    /// The resize has finished, and the surface needs to commit its final state.
    WaitingForCommit(ResizeData),
}

impl Default for ResizeState {
    fn default() -> Self {
        Self::Idle
    }
}
