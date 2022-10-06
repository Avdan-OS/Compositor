#![allow(unused_parens)]

use super::state::AvCompositor;

use smithay::{
    desktop::{
        Kind,
        Space,
        Window,
        WindowSurfaceType,
    },
    input::pointer::{
        AxisFrame,
        ButtonEvent,
        GrabStartData,
        MotionEvent,
        PointerGrab,
        PointerInnerHandle,
    },
    reexports::{
        wayland_protocols::xdg::shell::server::xdg_toplevel,
        wayland_server::protocol::wl_surface::WlSurface,
    },
    utils::{
        Logical,
        Point,
        Rectangle,
        Size,
    },
    wayland::{
        compositor::{self, SurfaceData},
        shell::xdg::{
            SurfaceCachedState,
            ToplevelState,
        },
    }
};

use std::cell::{
    RefCell,
    RefMut,
};

bitflags::bitflags! {
    pub struct ResizeEdge: u32 {
        const TOP          = 0b0001;
        const BOTTOM       = 0b0010;
        const LEFT         = 0b0100;
        const RIGHT        = 0b1000;

        const TOP_LEFT     = Self::TOP.bits | Self::LEFT.bits;
        const BOTTOM_LEFT  = Self::BOTTOM.bits | Self::LEFT.bits;

        const TOP_RIGHT    = Self::TOP.bits | Self::RIGHT.bits;
        const BOTTOM_RIGHT = Self::BOTTOM.bits | Self::RIGHT.bits;
    }
}

pub struct MoveSurfaceGrab {
    pub start_data:              GrabStartData<AvCompositor>,
    pub window:                  Window,
    pub initial_window_location: Point<i32, Logical>,
}

pub struct ResizeSurfaceGrab {
    start_data: GrabStartData<AvCompositor>,
    window:     Window,

    edges: ResizeEdge,

    initial_rect:     Rectangle<i32, Logical>,
    last_window_size: Size<i32, Logical>,
}

impl PointerGrab<AvCompositor> for MoveSurfaceGrab {
    fn motion (
        &mut self,
        data:   &mut AvCompositor,
        handle: &mut PointerInnerHandle<'_, AvCompositor>,
        _focus: Option<(WlSurface, Point<i32, Logical>)>,
        event:  &MotionEvent,
    ) {
        handle.motion(data, None, event);

        let delta       : Point<f64, Logical> = event.location - self.start_data.location;
        let new_location: Point<f64, Logical> = self.initial_window_location.to_f64() + delta;

        data.space
            .map_window(&self.window, new_location.to_i32_round(), None, true);
    }

    fn button (
        &mut self,
        data:   &mut AvCompositor,
        handle: &mut PointerInnerHandle<'_, AvCompositor>,
        event:  &ButtonEvent,
    ) {
        handle.button(data, event);

        const BTN_LEFT: u32 = 0x110;

        if (!handle.current_pressed().contains(&BTN_LEFT)) {
            handle.unset_grab(data, event.serial, event.time);
        }
    }

    fn axis (
        &mut self,
        data:    &mut AvCompositor,
        handle:  &mut PointerInnerHandle<'_, AvCompositor>,
        details: AxisFrame,
    ) {
        handle.axis(data, details)
    }

    fn start_data(&self) -> &GrabStartData<AvCompositor> {
        &self.start_data
    }
}

impl From<xdg_toplevel::ResizeEdge> for ResizeEdge {
    #[inline]
    fn from(x: xdg_toplevel::ResizeEdge) -> Self {
        Self::from_bits(x as u32).unwrap()
    }
}

impl ResizeSurfaceGrab {
    pub fn start(
        start_data:          GrabStartData<AvCompositor>,
        window:              Window,
        edges:               ResizeEdge,
        initial_window_rect: Rectangle<i32, Logical>,
    ) -> Self {
        let initial_rect: Rectangle<i32, Logical> = initial_window_rect;

        ResizeSurfaceState::with (
                window.toplevel().wl_surface(),
                |state: &mut ResizeSurfaceState| {
            *state = ResizeSurfaceState::Resizing {
                edges,
                initial_rect
            };
        });

        Self {
            start_data,
            window,
            edges,
            initial_rect,
            last_window_size: initial_rect.size,
        }
    }
}

impl PointerGrab<AvCompositor> for ResizeSurfaceGrab {
    fn motion (
        &mut self,
        data:   &mut AvCompositor,
        handle: &mut PointerInnerHandle<'_, AvCompositor>,
        _focus: Option<(WlSurface, Point<i32, Logical>)>,
        event:  &MotionEvent,
    ) {
        handle.motion(data, None, event);

        let mut delta: Point<f64, Logical> = event.location - self.start_data.location;

        let mut new_window_width:  i32 = self.initial_rect.size.w;
        let mut new_window_height: i32 = self.initial_rect.size.h;

        if (self.edges.intersects(ResizeEdge::LEFT | ResizeEdge::RIGHT)) {
            if (self.edges.intersects(ResizeEdge::LEFT)) {
                delta.x = -delta.x;
            }

            new_window_width = (self.initial_rect.size.w as f64 + delta.x) as i32;
        }

        if (self.edges.intersects(ResizeEdge::TOP | ResizeEdge::BOTTOM)) {
            if (self.edges.intersects(ResizeEdge::TOP)) {
                delta.y = -delta.y;
            }

            new_window_height = (self.initial_rect.size.h as f64 + delta.y) as i32;
        }

        let (min_size, max_size): (Size<i32, Logical>, Size<i32, Logical>) =
                compositor::with_states (
                    self.window.toplevel().wl_surface(),
                    |states: &SurfaceData| {
            let data: RefMut<SurfaceCachedState> = states.cached_state.current::<SurfaceCachedState>();

            (data.min_size, data.max_size)
        });

        let min_width:  i32 = min_size.w.max(1);
        let min_height: i32 = min_size.h.max(1);

        let max_width:  i32 = (max_size.w == 0).then(i32::max_value).unwrap_or(max_size.w);
        let max_height: i32 = (max_size.h == 0).then(i32::max_value).unwrap_or(max_size.h);

        self.last_window_size = Size::from((
            new_window_width.max(min_width).min(max_width),
            new_window_height.max(min_height).min(max_height),
        ));

        if let Kind::Xdg(xdg) = self.window.toplevel() {
            xdg.with_pending_state(|state: &mut ToplevelState| {
                state.states.set(xdg_toplevel::State::Resizing);
                state.size = Some(self.last_window_size);
            });

            xdg.send_configure();
        }
    }

    fn button (
        &mut self,
        data:   &mut AvCompositor,
        handle: &mut PointerInnerHandle<'_, AvCompositor>,
        event:  &ButtonEvent,
    ) {
        handle.button(data, event);

        const BTN_LEFT: u32 = 0x110;

        if (!handle.current_pressed().contains(&BTN_LEFT)) {
            // No more buttons are pressed, release the grab.
            handle.unset_grab(data, event.serial, event.time);

            if let Kind::Xdg(xdg) = self.window.toplevel() {
                xdg.with_pending_state(|state: &mut ToplevelState| {
                    state.states.unset(xdg_toplevel::State::Resizing);
                    state.size = Some(self.last_window_size);
                });

                xdg.send_configure();

                ResizeSurfaceState::with(xdg.wl_surface(), |state: &mut ResizeSurfaceState| {
                    *state = ResizeSurfaceState::WaitingForLastCommit {
                        edges       : self.edges,
                        initial_rect: self.initial_rect,
                    };
                });
            }
        }
    }

    fn axis (
        &mut self,
        data:    &mut AvCompositor,
        handle:  &mut PointerInnerHandle<'_, AvCompositor>,
        details: AxisFrame,
    ) { handle.axis(data, details) }

    fn start_data(&self) -> &GrabStartData<AvCompositor> {
        &self.start_data
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ResizeSurfaceState {
    Idle,
    Resizing {
        edges       : ResizeEdge,
        /// The initial window size and location.
        initial_rect: Rectangle<i32, Logical>,
    },
    /// Resize is done, we are now waiting for last commit, to do the final move
    WaitingForLastCommit {
        edges       : ResizeEdge,
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
    fn with<F, T>(surface: &WlSurface, cb: F) -> T
        where
            F: FnOnce(&mut Self) -> T,
    {
        compositor::with_states(surface, |states: &SurfaceData| {
            states.data_map.insert_if_missing(RefCell::<Self>::default);

            let state: &RefCell<ResizeSurfaceState> = states.data_map.get::<RefCell<Self>>().unwrap();

            cb(&mut *state.borrow_mut())
        })
    }

    #[allow(dead_code)]
    fn commit(&mut self) -> Option<(ResizeEdge, Rectangle<i32, Logical>)> {
        match *self {
            Self::Resizing {
                edges,
                initial_rect
            } => Some((edges, initial_rect)),

            Self::WaitingForLastCommit {
                edges,
                initial_rect
            } => {
                // The resize is done, let's go back to idle
                *self = Self::Idle;

                Some((edges, initial_rect))
            }

            Self::Idle => None,
        }
    }
}

#[allow(dead_code)]
pub fn handle_commit (
    space  : &mut Space,
    surface: &WlSurface
) -> Option<()> {
    let window: Window = space
        .window_for_surface(surface, WindowSurfaceType::TOPLEVEL)
        .cloned()?;

    let mut window_location: Point<i32, Logical> = space.window_location(&window)?;
    let geometry        : Rectangle<i32, Logical> = window.geometry();

    let new_loc: Point<Option<i32>, Logical> = ResizeSurfaceState::with (
            surface,
            |state: &mut ResizeSurfaceState| {
        state
            .commit()
            .and_then(|(edges, initial_rect)| {
                // If the window is being resized by top or left, its location must be adjusted
                // accordingly.
                edges.intersects(ResizeEdge::TOP_LEFT).then(|| {
                    let new_x: Option<i32> = edges
                        .intersects(ResizeEdge::LEFT)
                        .then(|| initial_rect.loc.x + (initial_rect.size.w - geometry.size.w));

                    let new_y: Option<i32> = edges
                        .intersects(ResizeEdge::TOP)
                        .then(|| initial_rect.loc.y + (initial_rect.size.h - geometry.size.h));

                    (new_x, new_y).into()
                })
            })
            .unwrap_or_default()
    });

    if let Some(new_x) = new_loc.x {
        window_location.x = new_x;
    }
    if let Some(new_y) = new_loc.y {
        window_location.y = new_y;
    }

    if (new_loc.x.is_some() || new_loc.y.is_some()) {
        // If TOP or LEFT side of the window got resized, we have to move it
        space.map_window(&window, window_location, None, false);
    }

    Some(())
}
