use smithay::{
    backend::renderer::{
        element::{
            surface::WaylandSurfaceRenderElement,
            utils::{
                CropRenderElement, RelocateRenderElement,
                RescaleRenderElement,
            },
            RenderElementStates, 
        }, 
        ImportAll, self,
        damage::{
            DamageTrackedRenderer, DamageTrackedRendererError
        },
        Renderer
    },
    output::Output,
    desktop::{
        Space, Window,
    },
    utils::{Rectangle, Physical}
};

use crate::compositor::components::cursor::PointerRenderElement;


renderer::element::render_elements! {
    pub CustomRenderElements<R> where
        R: ImportAll;
    Pointer=PointerRenderElement<R>,
    Surface=WaylandSurfaceRenderElement<R>
}

renderer::element::render_elements! {
    pub OutputRenderElements<'a, R> where
        R: ImportAll;
        
    Custom=&'a CustomRenderElements<R>,
    Preview=CropRenderElement<RelocateRenderElement<RescaleRenderElement<WaylandSurfaceRenderElement<R>>>>,
}

///
/// TODO: this is broke
/// 
#[allow(unused)]
fn render_output<'a, R>(
    output: &Output,
    space: &'a Space<Window>,
    custom_elements: &'a [CustomRenderElements<R>],
    renderer: &mut R,
    damage_tracked_renderer: &mut DamageTrackedRenderer,
    age: usize,
    show_window_preview: bool,
    log: &slog::Logger,
) -> Result<(Option<Vec<Rectangle<i32, Physical>>>, RenderElementStates), DamageTrackedRendererError<R>>
where
    R: Renderer + ImportAll,
    R::TextureId: Clone + 'static,
{
    // This original Anvil function does really cool fancy
    // previews of each window.
    // 
    // I can't be bothered right now to include this, 
    // so instead, here's this long comment.
    todo!()
}