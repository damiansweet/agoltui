mod app;
#[path = "../../src/auth.rs"]
mod auth;
mod data;

use app::AgolGui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "AGOL GUI",
        options,
        Box::new(|creation_context| Ok(Box::new(AgolGui::new(creation_context)))),
    )
}
