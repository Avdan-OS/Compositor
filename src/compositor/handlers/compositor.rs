use smithay::{wayland::{compositor::{CompositorHandler, CompositorState, is_sync_subsurface, get_parent}, buffer::BufferHandler, shm::{ShmHandler, ShmState}}, reexports::wayland_server::protocol::{wl_surface::WlSurface, wl_buffer::WlBuffer}, backend::renderer::utils::on_commit_buffer_handler, delegate_compositor, delegate_shm};

use crate::compositor::{state::Navda, grabs::resize_grab};

use super::xdg_shell;

impl CompositorHandler for Navda {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn commit(
        &mut self,
        surface: &WlSurface
    ) {
        on_commit_buffer_handler(surface);
        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self.space.elements()
                .find(|w| w.toplevel()
                .wl_surface() == &root)
            {
                window.on_commit();
            }
        };
        
        xdg_shell::handle_commit(&self.space, surface);
        resize_grab::handle_commit(&mut self.space, surface);
    }
}

impl BufferHandler for Navda {
    fn buffer_destroyed(&mut self, buffer: &WlBuffer) {}
}

impl ShmHandler for Navda {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}

delegate_compositor!(Navda);
delegate_shm!(Navda);