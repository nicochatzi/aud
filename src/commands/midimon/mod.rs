mod app;
mod ui;

use ratatui::prelude::*;

pub fn run<B: Backend>(terminal: &mut Terminal<B>) -> anyhow::Result<()> {
    crate::app::run::<app::App, B, _>(terminal, ui::render)
}
