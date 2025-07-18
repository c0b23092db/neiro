use crate::interactive::app::App;
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
use anyhow::Result;

pub fn run_interactive_player(file_path:&str, volume:u8) -> Result<()> {
    enable_raw_mode()?;
    let mut terminal = ratatui::init();
    let result = App::new().run(&mut terminal, file_path, volume);
    disable_raw_mode()?;
    ratatui::restore();
    result
}