#![feature(cell_update)]
use ui::initialize_ui;

mod daemon;
mod ui;

fn main() {
    initialize_ui();
}
