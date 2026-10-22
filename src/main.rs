#![forbid(unsafe_code)]

mod analysis;
mod app;
mod ballot;
mod confirm;
mod flow_sheet;
mod guide;
mod hero;
mod markdown;
mod model;
mod openrouter;
mod persistence;
mod platform;
mod progress;
mod round_save;
mod setup_card;
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
