use std::cell::RefCell;

use bitflags::bitflags;
use smithay::{
    input::pointer::{
        GrabStartData as PointerGrabStartData, PointerGrab, PointerInnerHandle, MotionEvent, ButtonEvent, AxisFrame
    },
    reexports::{wayland_protocols::xdg::{shell::server::xdg_toplevel, self}, wayland_server::protocol::wl_surface::WlSurface}, desktop::{Window, Kind, Space}, utils::{Rectangle, Logical, Point, Size}, wayland::{compositor, shell::xdg::SurfaceCachedState}
};

use crate::compositor::{state::Navda,};

bitflags! {
   pub struct ResizeEdge : u32 {
    const TOP       = 0b0001;
    const BOTTOM    = 0b0010;
    const LEFT      = 0b0100;
    const RIGHT     = 0b1000;

    const TOP_LEFT  = Self::TOP.bits | Self::LEFT.bits;
    const BOTTOM_LEFT  = Self::BOTTOM.bits | Self::LEFT.bits;
    const TOP_RIGHT  = Self::TOP.bits | Self::RIGHT.bits;
    const BOTTOM_RIGHT  = Self::BOTTOM.bits | Self::RIGHT.bits;
   } 
}

impl From<xdg_toplevel::ResizeEdge> for ResizeEdge {
    #[inline]
    fn from(x: xdg_toplevel::ResizeEdge) -> Self {
        Self::from_bits(x as u32).unwrap()
    }
}

pub struct ResizeSurfaceGrab {
    start_data  : PointerGrabStartData<Navda>,
    window      : Window,

    edges       : ResizeEdge,

    initial_rect: Rectangle<i32, Logical>,
    
    last_window_size : Size<i32, Logical>
}


impl ResizeSurfaceGrab {
    pub fn start(
        start_data  : PointerGrabStartData<Navda>,
        window      : Window,
        edges       : ResizeEdge,

        initial_window_rect : Rectangle<i32, Logical>,
    ) -> Self {
        let initial_rect = initial_window_rect;

        ResizeSurfaceState::with(
            window.toplevel().wl_surface(),
            |state| {
                *state = ResizeSurfaceState::Resizing { edges, initial_rect };
            }
        );

        Self {
            start_data,
            window,
            edges,
            initial_rect,
            last_window_size : initial_rect.size
        }
    }
}

impl PointerGrab<Navda> for ResizeSurfaceGrab {
    fn motion(
        &mut self,
        data: &mut Navda,
        handle: &mut PointerInnerHandle<'_, Navda>,
        focus: Option<(WlSurface, Point<i32, Logical>)>,
        event: &MotionEvent,
    ) {
        // No client with any pointer focus, again.
        handle.motion(data, None, event);

        let mut delta = event.location - self.start_data.location;

        let mut new_window_width  = self.initial_rect.size.w;
        let mut new_window_height = self.initial_rect.size.h;

        if self.edges.intersects(
            ResizeEdge::LEFT | ResizeEdge::RIGHT
        ) {
            if self.edges.intersects(ResizeEdge::LEFT) {
                delta.x = -delta.x;
            }

            new_window_width = (self.initial_rect.size.w as f64 + delta.x) as i32;
        }

        if self.edges.intersects(
            ResizeEdge::TOP | ResizeEdge::BOTTOM
        ) {
            if self.edges.intersects(ResizeEdge::TOP) {
                delta.y = -delta.y;
            }

            new_window_height = (self.initial_rect.size.h as f64 + delta.y) as i32;
        }

        let (min_size, max_size) = compositor::with_states(
            self.window.toplevel().wl_surface(),
            |states| {
                let data = states.cached_state.current::<SurfaceCachedState>();
                (data.min_size, data.max_size)
            }
        );

        let min_width = min_size.w.max(1);
        let min_height = min_size.h.max(1);

        let max_width = (max_size.w == 0)
            .then(|| i32::MAX).unwrap_or(max_size.w);

        let max_height = (max_size.h == 0)
            .then(|| i32::MAX).unwrap_or(max_size.h);

        self.last_window_size = Size::from((
            new_window_width.max(min_width).min(max_width),
            new_window_height.max(min_height).min(max_height),
        ));

        if let Kind::Xdg(xdg) = self.window.toplevel() {
            xdg.with_pending_state(|state| {
                state.states.set(xdg_toplevel::State::Resizing);
                state.size = Some(self.last_window_size);
            });

            xdg.send_configure();
        }
    }

    fn button(
        &mut self,
        data: &mut Navda,
        handle: &mut PointerInnerHandle<'_, Navda>,
        event: &ButtonEvent
    ) {
        handle.button(data, event);    

        // See `move_grab.rs` for link to kernel header.
        const BTN_LEFT : u32 = 0x110;

        if !handle.current_pressed().contains(&BTN_LEFT) {
            // No longer held down, release our grab.
            handle.unset_grab(data, event.serial, event.time);

            if let Kind::Xdg(xdg) = self.window.toplevel() {
                xdg.with_pending_state(|state| {
                    state.states.unset(xdg_toplevel::State::Resizing);
                    state.size = Some(self.last_window_size);
                });

                xdg.send_configure();

                ResizeSurfaceState::with(xdg.wl_surface(), |state| {
                    *state = ResizeSurfaceState::WaitingForLastCommit {
                        edges: self.edges,
                        initial_rect: self.initial_rect
                    };
                })
            }
        }
    }

    fn axis(
        &mut self,
        data: &mut Navda,
        handle: &mut PointerInnerHandle<'_, Navda>,
        details: AxisFrame
    ) {
        handle.axis(data, details);    
    }

    fn start_data(&self) -> &PointerGrabStartData<Navda> {
        &self.start_data
    }
}

/// 
/// From Smithay/smallvil
/// "State of the resize operation.""
///
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ResizeSurfaceState {
    Idle,
    Resizing {
        edges: ResizeEdge,

        /// Initial window's size and location.
        initial_rect: Rectangle<i32, Logical>,
    },

    /// Resize is done, we are now waiting for last commit, to do the final move
    WaitingForLastCommit {
        edges: ResizeEdge,
    
        /// The initial window size and location.
        initial_rect: Rectangle<i32, Logical>,
    },
}

impl Default for ResizeSurfaceState {
    fn default() -> Self {
        ResizeSurfaceState::Idle
    }
}

impl ResizeSurfaceState {
    fn with<F, T>(
        surface : &WlSurface,
        cb : F,
    ) -> T
        where
            F: FnOnce(&mut Self) -> T,
    {
        compositor::with_states(surface, |states| {
            states.data_map.insert_if_missing(
                RefCell::<Self>::default
            );

            let state = states.data_map.get::<RefCell<Self>>().unwrap();

            cb(&mut state.borrow_mut())
        })
    }

    fn commit(&mut self) -> Option<(ResizeEdge, Rectangle<i32, Logical>)> {
        match *self {
            Self::Resizing { edges, initial_rect }
                => Some((edges, initial_rect)),
            
            Self::WaitingForLastCommit { edges, initial_rect }
                => {
                    *self = Self::Idle;

                    Some((edges, initial_rect))
                },
            
            Self::Idle => None
        }
    }
}

pub fn handle_commit(
    space   : &mut Space<Window>,
    surface : &WlSurface
) -> Option<()> {
    let window = space
        .elements()
        .find(|w| w.toplevel().wl_surface() == surface)
        .cloned()?;

    let mut window_loc = space.element_location(&window)?;
    let geometry = window.geometry();

    let new_loc : Point<Option<i32>, Logical> = ResizeSurfaceState::with(
        surface, |state| {
            state
                .commit()
                .and_then(|(edges, initial_rect)| {
                    // Adjust location for top and/or left edge resizing.
                    edges.intersects(ResizeEdge::TOP_LEFT).then(|| {
                        let new_x = edges
                            .intersects(ResizeEdge::LEFT)
                            .then_some(initial_rect.loc.x + (initial_rect.size.w - geometry.size.w));
                        
                            let new_y= edges
                            .intersects(ResizeEdge::TOP)
                            .then_some(initial_rect.loc.y + (initial_rect.size.h - geometry.size.h));

                            (new_x, new_y).into()
                        })
                })
                .unwrap_or_default()
        }
    );

    if let Some(new_x) = new_loc.x {
        window_loc.x = new_x;
    }
    if let Some(new_y) = new_loc.y {
        window_loc.y = new_y;
    }

    if new_loc.x.is_some() || new_loc.y.is_some() {
        // If window LEFT or TOP got resized -> update location.
        space.map_element(window, window_loc, false);
    }

    Some(())
}