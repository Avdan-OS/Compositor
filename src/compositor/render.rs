use smithay::{
    backend::renderer::{
        damage::{DamageTrackedRenderer, DamageTrackedRendererError, DamageTrackedRendererMode},
        element::{
            surface::WaylandSurfaceRenderElement, utils::CropRenderElement, AsRenderElements,
            RenderElementStates,
        },
        ImportAll, ImportMem, Renderer,
    },
    desktop::{self, Space},
    output::Output,
    render_elements,
    utils::{Physical, Rectangle},
};

use super::{
    drawing::{PointerRenderElement, CLEAR_COLOR},
    shell::{AvWindow, AvWindowRenderElement, FullscreenSurface},
};

render_elements! {
    pub CustomRenderElements<R> where
        R: ImportAll + ImportMem;
    Pointer=PointerRenderElement<R>,
    Surface=WaylandSurfaceRenderElement<R>,
    Window=AvWindowRenderElement<R>,
}

render_elements! {
    pub OutputRenderElements<'a, R> where
        R: ImportAll + ImportMem;
    Custom=&'a CustomRenderElements<R>,
}

///
/// Big boi render function -- render everything:
/// all elements; to the output.
///
pub fn render_output<'a, R>(
    output: &Output,
    space: &'a Space<AvWindow>,
    custom_elements: &'a [CustomRenderElements<R>],
    renderer: &mut R,
    damage_tracked_renderer: &mut DamageTrackedRenderer,
    age: usize,
    // NOTE: review code for window preview for the
    // "App view" thing.
    // show_window_preview: bool,
    log: &slog::Logger,
) -> Result<
    (Option<Vec<Rectangle<i32, Physical>>>, RenderElementStates),
    DamageTrackedRendererError<R>,
>
where
    R: Renderer + ImportAll + ImportMem,
    R::TextureId: Clone + 'static,
{
    let output_scale = output.current_scale().fractional_scale().into();

    if let Some(window) = output
        .user_data()
        .get::<FullscreenSurface>()
        .and_then(|f| f.get())
    {
        if let DamageTrackedRendererMode::Auto(renderer_output) = damage_tracked_renderer.mode() {
            assert!(renderer_output == output);
        }

        let window_render_elements =
            AsRenderElements::<R>::render_elements(&window, renderer, (0, 0).into(), output_scale);

        let render_elements = custom_elements
            .iter()
            .chain(window_render_elements.iter())
            .collect::<Vec<_>>();

        damage_tracked_renderer.render_output(
            renderer,
            age,
            &render_elements,
            CLEAR_COLOR,
            log.clone(),
        )
    } else {
        let output_render_elements = custom_elements
            .iter()
            .map(OutputRenderElements::from)
            .collect::<Vec<_>>();

        // SNIP : Window Previews

        desktop::space::render_output(
            output,
            renderer,
            age,
            [space],
            &output_render_elements,
            damage_tracked_renderer,
            CLEAR_COLOR,
            log.clone(),
        )
    }
}
