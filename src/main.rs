#![forbid(unsafe_code)]

mod app;
mod model;
mod progress;
mod shorthand;
mod state;
mod timer;
mod topic;

fn main() {
    // First: with panic = "abort" a panic otherwise traps with only
    // `unreachable executed`; the hook prints the Rust panic message
    // (and a backtrace hint) to the browser console before the trap.
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
