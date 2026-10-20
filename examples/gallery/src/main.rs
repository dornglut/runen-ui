//! Headless Gallery shell smoke runner. The native live-preview host is not yet implemented.
use gallery::{GalleryApp, GalleryState, style_environment};
use runenui_core::LogicalLength;
use runenui_runtime::{
    AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext, render_debug_surface_frame,
};

fn main() {
    let mut gallery = AppRuntime::<GalleryApp>::mount(GalleryState::default());
    gallery
        .pump(PumpBudget::new(256, 256, 256, 256))
        .unwrap_or_else(|_| unreachable!("gallery initialization is bounded"));
    let style = style_environment();
    let context = SurfaceBuildContext::tight(
        &style,
        LogicalSize::new(LogicalLength::from(760_u16), LogicalLength::from(520_u16)),
    );
    let publication = gallery
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("gallery default surface publishes"));
    println!("{}", render_debug_surface_frame(publication.frame()));
}
