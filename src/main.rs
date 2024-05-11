static APP_NAME: &str = "Meticulous";

use eframe::egui;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(APP_NAME, options, Box::new(|_cc| Box::new(App::default())))
        .expect("failed to start app");
}

#[cfg(target_arch = "wasm32")]
fn main() {
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions {
        ..Default::default()
    };
    wasm_bindgen_futures::spawn_local(async {
        eframe::WebRunner::new()
            .start(
                APP_NAME, // hardcode it
                web_options,
                Box::new(|_cc| Box::new(App::default())),
            )
            .await
            .expect("failed to start eframe");
    });
}

#[derive(Debug, Default)]
struct App {
    count: usize,
}

impl eframe::App for App {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading(APP_NAME);
            ui.label("HELLO");
            if ui.button(format!("hello {}", self.count)).clicked() {
                self.count += 1;
            }
        });
    }
}
