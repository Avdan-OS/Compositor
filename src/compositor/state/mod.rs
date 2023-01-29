//!
//! Collection of state structs for the compositor.
//!

use std::sync::{atomic::AtomicBool, Arc, Mutex};

use smithay::{
    delegate_compositor,
    desktop::{PopupManager, Space},
    input::{pointer::CursorImageStatus, Seat, SeatState},
    reexports::{
        calloop::LoopHandle,
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::wl_surface::WlSurface,
            Display, DisplayHandle,
        },
    },
    utils::{Clock, Logical, Monotonic, Point},
    wayland::{
        compositor::CompositorState,
        data_device::DataDeviceState,
        fractional_scale::FractionalScaleManagerState,
        keyboard_shortcuts_inhibit::KeyboardShortcutsInhibitState,
        output::OutputManagerState,
        presentation::PresentationState,
        primary_selection::PrimarySelectionState,
        shell::{
            wlr_layer::WlrLayerShellState,
            xdg::{decoration::XdgDecorationState, XdgShellState},
        },
        shm::ShmState,
        viewporter::ViewportState,
        xdg_activation::XdgActivationState,
    },
    xwayland::{X11Wm, XWayland},
};

use super::backend::Backend;

///
/// State for a client (application).
///
#[derive(Debug, Default)]
pub struct ClientState;
impl ClientData for ClientState {
    /// Notification that a client was initialized
    fn initialized(&self, _client_id: ClientId) {}
    /// Notification that a client is disconnected
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}

///
/// State for callbacks in the event loop.
///
pub struct CalloopData<BEnd: Backend + 'static> {
    pub state: Navda<BEnd>,
    pub display: Display<Navda<BEnd>>,
}

#[derive(Debug)]
///
/// State for Navda's Wayland compositor.
///
pub struct Navda<BEnd: Backend + 'static> {
    ///
    /// State associated with a particular backend
    /// (e.g. `udev`, `winit`)
    ///
    pub backend_data: BEnd,

    ///
    /// Wayland socket name (e.g. "wayland-0").
    ///
    /// We'll always be listening, so no Option<...>
    /// like anvil.
    ///
    pub socket_name: String,

    ///
    /// Clone-able handle for the Wayland Display.
    ///
    pub display_handle: DisplayHandle,

    ///
    /// Is the compositor running?
    ///
    pub running: Arc<AtomicBool>,

    ///
    /// Handle to the main event loop.
    ///
    pub handle: LoopHandle<'static, CalloopData<BEnd>>,

    // <DESKTOP>
    ///
    /// 2D Plane for elements to map upon.
    ///
    /// Things like Windows, Widgets can live
    /// here.
    ///
    pub space: Space<AvWindow>,

    ///
    /// Helper for popups.
    ///
    pub popups: PopupManager,

    // </DESKTOP>

    // <SMITHAY>
    ///
    /// Smithay's internal state for compositors.
    ///
    pub compositor_state: CompositorState,

    ///
    /// Handles state of data devices.
    ///
    /// Data devices are Wayland's answer to clipboards
    /// and drag-and-drop.
    ///
    /// See the [Wayland Documentation](https://wayland.freedesktop.org/docs/html/ch04.html#:~:text=Data%20devices%20glue%20data)
    ///
    pub data_device_state: DataDeviceState,

    ///
    /// Allows for the request of all known
    /// surfaces to the shell.
    ///
    pub layer_shell_state: WlrLayerShellState,

    ///
    /// Smithay's state for managing outputs
    /// with the XDG spec.
    ///
    /// See more about xdg_output_manager
    /// on [Wayland.app](https://wayland.app/protocols/xdg-output-unstable-v1).
    ///
    pub output_manager_state: OutputManagerState,

    ///
    /// State for the primary selection.
    ///
    /// See more about the PRIMARY selection
    /// on the [Arch Wiki](https://wiki.archlinux.org/title/Clipboard#:~:text=PRIMARY).
    ///
    pub primary_selection_state: PrimarySelectionState,

    ///
    /// State for all Seat globals.
    ///
    /// See `Navda.seat` for more information on seats.
    ///
    pub seat_state: SeatState<Self>,

    ///
    /// Smithay state for the Keyboard shortcuts inhibit
    /// protocol.
    ///
    /// > "This protocol specifies a way for a client to request the compositor
    /// > to ignore its own keyboard shortcuts for a given seat,
    /// > so that all key events from that seat get forwarded to a surface."
    ///
    /// See more on [Wayland.app](https://wayland.app/protocols/keyboard-shortcuts-inhibit-unstable-v1)
    ///
    pub keyboard_shortcuts_inhibit_state: KeyboardShortcutsInhibitState,

    ///
    /// Smithay state for handling
    /// Shared Memory (shm) between the compositor
    /// and clients.
    ///
    /// See more on [The Wayland Book](https://wayland-book.com/surfaces/shared-memory.html).
    ///
    pub shm_state: ShmState,

    // TODO (@Sammy99jsp) Review need for this state.
    ///
    /// Smithay state for the Viewporter protocol.
    ///
    /// The wp_viewporter interface allows for an extension
    /// interface for `wl_surface` (Wayland Surface) objects
    /// which allows them to be cropped and scaled.  
    ///
    /// See more on [Wayland.app](https://wayland.app/protocols/viewporter).
    ///
    pub viewporter_state: ViewportState,

    ///
    /// Smithay state for the XDG activation protocol,
    /// which allows clients to pass focus to another toplevel surface.
    ///
    /// See more on [Wayland.app](https://wayland.app/protocols/xdg-activation-v1).
    ///
    pub xdg_activation_state: XdgActivationState,

    ///
    /// Smithay state for the XDG decoration protocol,
    /// which allows for a compositor to notify clients that
    /// it supports drawing window decorations (e.g. title bar, minimize, maximize, etc.)
    ///
    /// See more on [Wayland.app](https://wayland.app/protocols/xdg-decoration-unstable-v1).
    ///
    ///
    pub xdg_decoration_state: XdgDecorationState,

    ///
    /// Smithay state for XDG Shell protocol,
    /// which outlines some components typically
    /// associated with desktop OS Shells (e.g. Windows, Popups, etc.)
    ///
    /// See more on [Wayland.app](https://wayland.app/protocols/xdg-shell).
    ///
    pub xdg_shell_state: XdgShellState,

    ///
    /// Smithay state for the Presentation time protocol,
    /// which allows for the compositor to give feedback to surfaces
    /// to ensure that video playback is smooth and in sync with audio.
    ///
    /// See more on [Wayland.app](https://wayland.app/protocols/presentation-time).
    ///
    pub presentation_state: PresentationState,

    ///
    /// Smithay state for the Fractional scale protocol,
    /// which allows the compositor to suggest for surfaces
    /// to render at a fractional scales (e.g. 1.5, 0.75, etc.);.
    ///
    pub fractional_scale_manager_state: FractionalScaleManagerState,

    // </SMITHAY>
    ///
    /// Logger for the compositor.
    ///
    pub log: slog::Logger,

    // <INPUT>

    // TODO(Sammy99jsp) : `Navda.suppressed_keys` description.
    ///
    /// ...
    ///
    pub suppressed_keys: Vec<u32>,

    ///
    /// Location of the pointer.
    ///
    pub pointer_location: Point<f64, Logical>,

    ///
    /// Status of the cursor:
    /// * Hidden
    /// * Default (compositor's own cursor)
    /// * Surface &mdash; the compositor should render
    /// this surface as the cursor.
    ///
    pub cursor_status: Arc<Mutex<CursorImageStatus>>,

    // TODO(Sammy99jsp) : `Navda.seat_name` description.
    ///
    pub seat_name: String,

    ///
    /// Smithay Seat handle.
    ///
    /// Wayland seats can abstract over a group of
    /// up to one keyboard and pointer device each
    /// (and other configs for different devices).
    ///
    /// See more in [The Wayland Book](https://wayland-book.com/seat.html)
    ///
    ///
    // NOTE: Multiple keyboard support, lol ?
    pub seat: Seat<Self>,

    ///
    /// Compositor's Monotonic (no jumps in time)
    /// Clock.
    ///
    pub clock: Clock<Monotonic>,

    ///
    /// Icon for drag and drop.
    ///
    pub dnd_icon: Option<WlSurface>,

    // </INPUT>

    // <XWAYLAND>
    ///
    ///
    /// Handle for XWayland,
    /// which allows Wayland to interop
    /// with X11 Clients.
    ///
    /// See more [here](https://wayland.freedesktop.org/xserver.html).
    ///
    pub xwayland: XWayland,

    ///
    /// Runtime state of the XWayland window manager.
    ///
    /// See more on Window Managers on the
    /// [Arch Wiki](https://wiki.archlinux.org/title/window_manager).  
    ///
    pub xwm: Option<X11Wm>,
    // </XWAYLAND>
}

delegate_compositor!(@<BEnd : Backend + 'static> Navda<BEnd>);
