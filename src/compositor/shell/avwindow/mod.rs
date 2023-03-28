//!
//! Contains various implementation for the
//! AvWindow abstraction over X11 (via XWayland) and
//! Wayland windows.
//!

mod state;

use std::time::Duration;

use smithay::{
    backend::renderer::{
        element::{surface::WaylandSurfaceRenderElement, AsRenderElements},
        ImportAll, ImportMem, Renderer, Texture,
    },
    desktop::{
        space::SpaceElement,
        utils::{
            send_frames_surface_tree, take_presentation_feedback_surface_tree,
            under_from_surface_tree, with_surfaces_surface_tree, OutputPresentationFeedback,
        },
        Window, WindowSurfaceType,
    },
    input::{
        keyboard::{KeyboardTarget, KeysymHandle},
        pointer::{self, PointerTarget},
        Seat,
    },
    output::Output,
    reexports::{
        wayland_protocols::wp::presentation_time::server::wp_presentation_feedback,
        wayland_server::protocol::wl_surface::WlSurface,
    },
    render_elements,
    utils::{user_data::UserDataMap, IsAlive, Logical, Physical, Point, Rectangle, Scale, Serial},
    wayland::{
        compositor::SurfaceData as WlSurfaceData, seat::WaylandFocus, shell::xdg::ToplevelSurface,
    },
    xwayland::X11Surface,
};

use crate::compositor::{backend::Backend, state::Navda};

///
/// Abstraction over X11 and Wayland Windows.
///
/// This would also later implement Tabbing,
/// and Tiling.
///
#[derive(Debug, PartialEq, Clone)]
pub enum AvWindow {
    ///
    /// Wayland Desktop Window
    ///
    Wayland(Window),

    ///
    /// XWayland 'Window'
    ///
    X11(X11Surface),
}

impl AvWindow {
    ///
    /// Find top-most surface with type `window_type`
    /// under this point.
    ///
    pub fn surface_under(
        &self,
        location: Point<f64, Logical>,
        window_type: WindowSurfaceType,
    ) -> Option<(WlSurface, Point<i32, Logical>)> {
        match self {
            Self::Wayland(w) => w.surface_under(location, window_type),
            Self::X11(w) => w
                .wl_surface()
                .and_then(|s| under_from_surface_tree(&s, location, (0, 0), window_type)),
        }
    }

    ///
    /// Run a function on all subsurfaces
    /// of this window.
    ///
    pub fn with_surfaces(&self, processor: impl FnMut(&WlSurface, &WlSurfaceData) + Copy) {
        match self {
            Self::Wayland(w) => w.with_surfaces(processor),
            Self::X11(w) => {
                if let Some(surface) = w.wl_surface() {
                    with_surfaces_surface_tree(&surface, processor);
                }
            }
        }
    }

    ///
    /// Send frame callback to surfaces in this window.
    ///
    pub fn send_frame(
        &self,
        output: &Output,
        time: impl Into<Duration>,
        throttle: Option<Duration>,
        primary_scan_out_output: impl FnMut(&WlSurface, &WlSurfaceData) -> Option<Output> + Copy,
    ) {
        match self {
            Self::Wayland(w) => w.send_frame(output, time, throttle, primary_scan_out_output),
            Self::X11(w) => {
                if let Some(surface) = w.wl_surface() {
                    send_frames_surface_tree(
                        &surface,
                        output,
                        time,
                        throttle,
                        primary_scan_out_output,
                    );
                }
            }
        }
    }

    ///
    /// Presentation feedback is an indication that a wl_surface's content
    /// update is now visible to the user.
    ///
    /// See more on presentation time feedback
    /// on [Wayland.app](https://wayland.app/protocols/presentation-time#wp_presentation_feedback).
    ///
    pub fn take_presentation_feedback(
        &self,
        output_feedback: &mut OutputPresentationFeedback,
        primary_scan_out_output: impl FnMut(&WlSurface, &WlSurfaceData) -> Option<Output> + Copy,
        presentation_feedback_flags: impl FnMut(&WlSurface, &WlSurfaceData) -> wp_presentation_feedback::Kind
            + Copy,
    ) {
        match self {
            Self::Wayland(w) => w.take_presentation_feedback(
                output_feedback,
                primary_scan_out_output,
                presentation_feedback_flags,
            ),
            Self::X11(w) => {
                if let Some(surface) = w.wl_surface() {
                    take_presentation_feedback_surface_tree(
                        &surface,
                        output_feedback,
                        primary_scan_out_output,
                        presentation_feedback_flags,
                    )
                }
            }
        }
    }

    // Utility functions

    ///
    /// Is this an X11 window running under XWayland ?
    ///  
    pub fn is_x11(&self) -> bool {
        matches!(self, Self::X11(_))
    }

    ///
    /// Is this a Wayland window ?
    ///
    pub fn is_wayland(&self) -> bool {
        matches!(self, Self::Wayland(_))
    }

    ///
    /// Returns the underlying wl_surface object,
    /// if any.
    ///
    pub fn wl_surface(&self) -> Option<WlSurface> {
        match self {
            Self::Wayland(w) => w.wl_surface(),
            Self::X11(w) => w.wl_surface(),
        }
    }

    ///
    /// Shows user_map data &mdash; a type-based
    /// key-value system for storing arbitrary info
    /// to do with this window.
    ///
    pub fn user_data(&self) -> &UserDataMap {
        match self {
            Self::Wayland(w) => w.user_data(),
            Self::X11(w) => w.user_data(),
        }
    }
}

///
/// Checks if the underlying window
/// is alive.
///
impl IsAlive for AvWindow {
    fn alive(&self) -> bool {
        match self {
            Self::Wayland(w) => w.alive(),
            Self::X11(w) => w.alive(),
        }
    }
}

// TODO(Sammy99jsp) Possibly automate `PointerTarget` and
//  `KeyboardTarget` with custom `#[pass(PointerTarget)]` macro

///
/// Allowing for `AvWindow`s to be interacted
/// with the pointer.
///
impl<BEnd: Backend> PointerTarget<Navda<BEnd>> for AvWindow {
    // TODO: Server-Side Decoration Logic
    fn enter(
        &self,
        seat: &Seat<Navda<BEnd>>,
        data: &mut Navda<BEnd>,
        event: &pointer::MotionEvent,
    ) {
        match self {
            Self::Wayland(w) => PointerTarget::enter(w, seat, data, event),
            Self::X11(w) => PointerTarget::enter(w, seat, data, event),
        }
    }

    /**
     *  TODO:
     *  `pass` macro idea:
     *  
     *  #[pass(PointerTarget)]
     *  fn enter(
     *      &self,
     *      seat : ...,
     *      data : ...,
     *      event: ...,
     *  )
     *
     */

    fn motion(
        &self,
        seat: &Seat<Navda<BEnd>>,
        data: &mut Navda<BEnd>,
        event: &pointer::MotionEvent,
    ) {
        match self {
            Self::Wayland(w) => PointerTarget::motion(w, seat, data, &event),
            Self::X11(w) => PointerTarget::motion(w, seat, data, event),
        }
    }

    fn button(
        &self,
        seat: &Seat<Navda<BEnd>>,
        data: &mut Navda<BEnd>,
        event: &pointer::ButtonEvent,
    ) {
        match self {
            Self::Wayland(w) => PointerTarget::button(w, seat, data, &event),
            Self::X11(w) => PointerTarget::button(w, seat, data, event),
        }
    }

    fn axis(&self, seat: &Seat<Navda<BEnd>>, data: &mut Navda<BEnd>, frame: pointer::AxisFrame) {
        match self {
            Self::Wayland(w) => PointerTarget::axis(w, seat, data, frame),
            Self::X11(w) => PointerTarget::axis(w, seat, data, frame),
        }
    }

    fn leave(&self, seat: &Seat<Navda<BEnd>>, data: &mut Navda<BEnd>, serial: Serial, time: u32) {
        match self {
            Self::Wayland(w) => PointerTarget::leave(w, seat, data, serial, time),
            Self::X11(w) => PointerTarget::leave(w, seat, data, serial, time),
        }
    }

    ///
    /// https://wayland.app/protocols/relative-pointer-unstable-v1
    ///
    fn relative_motion(
        &self,
        seat: &Seat<Navda<BEnd>>,
        data: &mut Navda<BEnd>,
        event: &pointer::RelativeMotionEvent,
    ) {
        match self {
            Self::Wayland(w) => PointerTarget::relative_motion(w, seat, data, event),
            Self::X11(w) => PointerTarget::relative_motion(w, seat, data, event),
        }
    }
}

///
/// Pass down keyboard events to window.
///
impl<BEnd: Backend> KeyboardTarget<Navda<BEnd>> for AvWindow {
    fn enter(
        &self,
        seat: &Seat<Navda<BEnd>>,
        data: &mut Navda<BEnd>,
        keys: Vec<KeysymHandle<'_>>,
        serial: Serial,
    ) {
        match self {
            Self::Wayland(w) => KeyboardTarget::enter(w, seat, data, keys, serial),
            Self::X11(w) => KeyboardTarget::enter(w, seat, data, keys, serial),
        }
    }

    fn leave(&self, seat: &Seat<Navda<BEnd>>, data: &mut Navda<BEnd>, serial: Serial) {
        match self {
            Self::Wayland(w) => KeyboardTarget::leave(w, seat, data, serial),
            Self::X11(w) => KeyboardTarget::leave(w, seat, data, serial),
        }
    }

    fn key(
        &self,
        seat: &Seat<Navda<BEnd>>,
        data: &mut Navda<BEnd>,
        key: KeysymHandle<'_>,
        state: smithay::backend::input::KeyState,
        serial: Serial,
        time: u32,
    ) {
        // TODO(@Sammy99jsp): Custom keybinds for switching tabs.

        match self {
            Self::Wayland(w) => KeyboardTarget::key(w, seat, data, key, state, serial, time),
            Self::X11(w) => KeyboardTarget::key(w, seat, data, key, state, serial, time),
        }
    }

    fn modifiers(
        &self,
        seat: &Seat<Navda<BEnd>>,
        data: &mut Navda<BEnd>,
        modifiers: smithay::input::keyboard::ModifiersState,
        serial: Serial,
    ) {
        // TODO(@Sammy99jsp): Custom keybinds for switching tabs.
        match self {
            AvWindow::Wayland(w) => KeyboardTarget::modifiers(w, seat, data, modifiers, serial),
            AvWindow::X11(w) => KeyboardTarget::modifiers(w, seat, data, modifiers, serial),
        }
    }
}

///
/// Allows the AvWindow to be mapped
/// in the 2D Space.
///
impl SpaceElement for AvWindow {
    ///
    /// Get the geometry for this window
    /// (aka) rectangle.
    ///
    fn geometry(&self) -> Rectangle<i32, Logical> {
        let geo = match self {
            Self::Wayland(w) => SpaceElement::geometry(w),
            Self::X11(w) => SpaceElement::geometry(w),
        };
        // TODO(Sammy99jsp) : Custom logic here for window decorations
        geo
    }

    ///
    /// Gets the bounding box for this window.
    ///
    fn bbox(&self) -> Rectangle<i32, Logical> {
        let bbox = match self {
            Self::Wayland(w) => SpaceElement::bbox(w),
            Self::X11(w) => SpaceElement::bbox(w),
        };
        // TODO(Sammy99jsp) : Custom logic here for window decorations.
        bbox
    }

    ///
    /// Checks if a point is in this window's
    /// input region.
    ///
    fn is_in_input_region(&self, point: &Point<f64, Logical>) -> bool {
        // TODO(Sammy99jsp) : Custom logic here
        //                     for window decorations.

        match self {
            AvWindow::Wayland(w) => SpaceElement::is_in_input_region(w, point),
            AvWindow::X11(w) => SpaceElement::is_in_input_region(w, point),
        }
    }

    ///
    /// Set rendered state to active, if applicable.
    ///
    fn set_activate(&self, activated: bool) {
        match self {
            Self::Wayland(w) => SpaceElement::set_activate(w, activated),
            Self::X11(w) => SpaceElement::set_activate(w, activated),
        }
    }

    ///
    /// Called when the window enters/outputs between outputs (aka. displays).
    ///
    fn output_enter(&self, output: &Output, overlap: Rectangle<i32, Logical>) {
        match self {
            Self::Wayland(w) => SpaceElement::output_enter(w, output, overlap),
            Self::X11(w) => SpaceElement::output_enter(w, output, overlap),
        }
    }

    ///
    /// Called when a window leaves a display.
    ///
    fn output_leave(&self, output: &Output) {
        match self {
            Self::Wayland(w) => SpaceElement::output_leave(w, output),
            Self::X11(w) => SpaceElement::output_leave(w, output),
        }
    }

    ///
    /// Get z-index of underlying display.
    ///
    fn z_index(&self) -> u8 {
        match self {
            Self::Wayland(w) => SpaceElement::z_index(w),
            Self::X11(w) => SpaceElement::z_index(w),
        }
    }

    ///
    /// Updates window's state &mdash; periodically called.
    ///
    fn refresh(&self) {
        match self {
            Self::Wayland(w) => SpaceElement::refresh(w),
            Self::X11(w) => SpaceElement::refresh(w),
        }
    }
}

render_elements!(
    pub AvWindowRenderElement<R> where R : ImportAll + ImportMem;
    Window=WaylandSurfaceRenderElement<R>,
);

impl<R> AsRenderElements<R> for AvWindow
where
    R: Renderer + ImportAll + ImportMem,
    <R as Renderer>::TextureId: Texture + 'static,
{
    type RenderElement = AvWindowRenderElement<R>;

    fn render_elements<C: From<Self::RenderElement>>(
        &self,
        renderer: &mut R,
        location: Point<i32, Physical>,
        scale: Scale<f64>,
    ) -> Vec<C> {
        let window_bbox = self.bbox();

        let window_geo = self.geometry();

        let width = window_geo.size.w;

        let mut vec = match self {
            Self::Wayland(xdg) => {
                AsRenderElements::<R>::render_elements(xdg, renderer, location, scale)
            }
            Self::X11(xdg) => {
                AsRenderElements::<R>::render_elements(xdg, renderer, location, scale)
            }
        };

        // TODO(@Sammy99jsp)
        // Custom rendering logic here for window decorations.

        vec.into_iter().map(C::from).collect()
    }
}
