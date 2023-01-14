use std::collections::HashMap;

use slog::warn;
use smithay::{
    backend::{
        renderer::{
            multigpu::{
                MultiRenderer, egl::EglGlesBackend,
                GpuManager, MultiTexture
            },
            gles2::{
                Gles2Renderer, Gles2Renderbuffer
            },
            element::texture::TextureBuffer
        },
        drm::DrmNode, 
        session::libseat::LibSeatSession
    },
    reexports::{
        drm::control::crtc, 
        wayland_server::{DisplayHandle, protocol::wl_surface}
    }, output::Output
};

use super::Backend;

type UdevRenderer<'a> =
    MultiRenderer<'a, 'a, EglGlesBackend<Gles2Renderer>, EglGlesBackend<Gles2Renderer>, Gles2Renderbuffer>;

#[derive(Debug, PartialEq)]
struct UdevOutputId {
    device_id: DrmNode,
    crtc: crtc::Handle,
}

pub struct UdevData {
    pub session: LibSeatSession,
    dh: DisplayHandle,
    primary_gpu: DrmNode,
    gpus: GpuManager<EglGlesBackend<Gles2Renderer>>,
    backends: HashMap<DrmNode, BackendData>,
    pointer_images: Vec<(xcursor::parser::Image, TextureBuffer<MultiTexture>)>,
    pointer_element: PointerElement<MultiTexture>,
    pointer_image: crate::compositor::components::Cursor,
    logger: slog::Logger,
}

impl Backend for UdevData {
    fn seat_name(&self) -> String {
        self.session.seat()
    }

    fn reset_buffers(&mut self, output: &Output) {
        if let Some(id) = output.user_data().get::<UdevOutputId>() {
            if let Some(gpu) = self.backends.get(&id.device_id) {
                let surfaces = gpu.surfaces.borrow();
                if let Some(surface) = surfaces.get(&id.crtc) {
                    surface.borrow_mut().surface.reset_buffers();
                }
            }
        }
    }

    fn early_import(&mut self, surface: &wl_surface::WlSurface) {
        if let Err(err) = self
            .gpus
            .early_import(Some(self.primary_gpu), self.primary_gpu, surface)
        {
            warn!(self.logger, "Early buffer import failed: {}", err);
        }
    }
}
