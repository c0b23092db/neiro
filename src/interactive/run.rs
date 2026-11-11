use crate::interactive::app::App;
use anyhow::Result;
use ratatui::{init, restore};

pub fn run_interactive_player(file_path:&str, volume:u8, timer:u64) -> Result<()> {
    // 画面を切り替える前に読み込み、エラーは通常の画面に表示する
    let mut app = App::new(file_path, volume)?;
    let mut terminal = init();
    let result = app.run(&mut terminal, timer);
    restore();
    result
}
