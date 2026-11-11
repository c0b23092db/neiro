use crate::interactive::app::App;
use anyhow::Result;
use ratatui::{init, restore};

pub fn run_interactive_player(file_path:&str, volume:u8, timer:u64) -> Result<()> {
    let mut terminal = init();
    let result = App::new().run(&mut terminal, file_path, volume, timer);
    restore();
    result
}