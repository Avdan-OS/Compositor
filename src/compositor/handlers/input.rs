//!
//! Collection of input handlers/Smithay delegations:
//! * [Data Device](https://wayland.freedesktop.org/docs/html/ch04.html#:~:text=Data%20devices%20glue%20data%20sources%20and%20offers%20together) Handler
//! * [Drag-and-drop](https://wayland.freedesktop.org/docs/html/ch04.html#:~:text=Drag%20and%20Drop)
//! * [Primary Selection](https://wiki.archlinux.org/title/Clipboard#:~:text=PRIMARY) Handler
//! * [Seat](https://wayland-book.com/seat.html) Handler
//! * [Tablet Manager (Tablet Protocol)](https://wayland.app/protocols/tablet-unstable-v2)
//! * [Text Input Manager (Text Input Protocol)](https://wayland.app/protocols/text-input-unstable-v3)
//! * [Input Method Manager (Input Method Protocol)](https://wayland.app/protocols/input-method-unstable-v1)
//! * [Keyboard Shortcut Inhibit State (Keyboard Shortcut Inhibit Protocol)](https://wayland.app/protocols/keyboard-shortcuts-inhibit-unstable-v1)
//!
//!

use std::os::unix::prelude::OwnedFd;

use smithay::{
    delegate_data_device, delegate_input_method_manager, delegate_keyboard_shortcuts_inhibit,
    delegate_primary_selection, delegate_seat, delegate_tablet_manager,
    delegate_text_input_manager, delegate_viewporter, delegate_virtual_keyboard_manager,
    input::{pointer::CursorImageStatus, Seat, SeatHandler, SeatState},
    reexports::wayland_server::protocol::{wl_data_source::WlDataSource, wl_surface::WlSurface},
    wayland::{
        data_device::{
            set_data_device_focus, ClientDndGrabHandler, DataDeviceHandler, DataDeviceState,
            ServerDndGrabHandler,
        },
        keyboard_shortcuts_inhibit::{
            KeyboardShortcutsInhibitHandler, KeyboardShortcutsInhibitState,
            KeyboardShortcutsInhibitor,
        },
        primary_selection::{set_primary_focus, PrimarySelectionHandler, PrimarySelectionState},
        seat::WaylandFocus,
    },
};

use crate::compositor::{backend::Backend, state::Navda};

impl<BEnd: Backend> SeatHandler for Navda<BEnd> {
    type KeyboardFocus = FocusTarget;
    type PointerFocus = FocusTarget;

    fn seat_state(&mut self) -> &mut SeatState<Self> {
        &mut self.seat_state
    }

    fn focus_changed(&mut self, seat: &Seat<Self>, target: Option<&FocusTarget>) {
        let dh = &self.display_handle;

        let focus = target
            .and_then(WaylandFocus::wl_surface)
            .and_then(|s| dh.get_client(s.id()).ok());
        set_data_device_focus(dh, seat, focus.clone());
        set_primary_focus(dh, seat, focus);
    }
    fn cursor_image(&mut self, _seat: &Seat<Self>, image: CursorImageStatus) {
        *self.cursor_status.lock().unwrap() = image;
    }
}
delegate_seat!(@<BEnd: Backend + 'static> Navda<BEnd>);

// Drag-and-drop
impl<BEnd: Backend> DataDeviceHandler for Navda<BEnd> {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
    fn send_selection(&mut self, _mime_type: String, _fd: OwnedFd) {
        todo!("Navda doesn't do server-side selections");
    }
}
impl<BEnd: Backend> ClientDndGrabHandler for Navda<BEnd> {
    fn started(
        &mut self,
        _source: Option<WlDataSource>,
        icon: Option<WlSurface>,
        _seat: Seat<Self>,
    ) {
        self.dnd_icon = icon;
    }

    fn dropped(&mut self, _seat: Seat<Self>) {
        self.dnd_icon = None;
    }
}
impl<BEnd: Backend> ServerDndGrabHandler for Navda<BEnd> {
    fn send(&mut self, _mime_type: String, _fd: OwnedFd) {
        todo!("Navda doesn't do server-side grabs");
    }
}

delegate_data_device!(@<BEnd: Backend + 'static> Navda<BEnd>);

// Primary Selection

impl<BackendData: Backend> PrimarySelectionHandler for Navda<BackendData> {
    fn primary_selection_state(&self) -> &PrimarySelectionState {
        &self.primary_selection_state
    }
}

delegate_primary_selection!(@<BEnd: Backend + 'static> Navda<BEnd>);

delegate_tablet_manager!(@<BEnd: Backend + 'static> Navda<BEnd>);

delegate_text_input_manager!(@<BEnd: Backend + 'static> Navda<BEnd>);

delegate_input_method_manager!(@<BEnd: Backend + 'static> Navda<BEnd>);

impl<BEnd: Backend> KeyboardShortcutsInhibitHandler for Navda<BEnd> {
    fn keyboard_shortcuts_inhibit_state(&mut self) -> &mut KeyboardShortcutsInhibitState {
        &mut self.keyboard_shortcuts_inhibit_state
    }

    fn new_inhibitor(&mut self, inhibitor: KeyboardShortcutsInhibitor) {
        // Just grant the wish for everyone
        inhibitor.activate();
    }
}

delegate_keyboard_shortcuts_inhibit!(@<BackendData: Backend + 'static> Navda<BackendData>);

delegate_virtual_keyboard_manager!(@<BackendData: Backend + 'static> Navda<BackendData>);

// TODO(Sammy99jsp) delegate_relative_pointer.
