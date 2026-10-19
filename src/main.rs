#![forbid(unsafe_code)]

mod app;
mod model;
mod state;
mod timer;

fn main() {
    leptos::mount::mount_to_body(app::App);
}
