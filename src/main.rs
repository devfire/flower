#![forbid(unsafe_code)]

mod app;
mod model;
mod state;

fn main() {
    leptos::mount::mount_to_body(app::App);
}
