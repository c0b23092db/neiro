use crate::interactive::app::App;
use crate::interactive::interactiveplayer::InteractivePlayer;
use ratatui::{
    style::{palette::tailwind, Stylize},
    text::{Line},
    widgets::{Block, Borders, Paragraph, Gauge},
    prelude::{Layout, Constraint},
    Frame,
};
use ratatui::prelude::*;

pub fn draw(frame: &mut Frame, _app: &App, player: &InteractivePlayer) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Length(1), // タイトル
            Constraint::Length(3), // ファイル名
            Constraint::Percentage(100),
            Constraint::Length(1), // 再生バー
            Constraint::Length(1), // 再生ボタン
            Constraint::Length(3), // 音量バー
        ])
        .split(frame.area());

    // タイトル //
    let title = Line::from("Neiro")
        .bold().blue().centered();
    frame.render_widget(Paragraph::new(title), layout[0]);
    // ファイル名 //
    let file_name = Paragraph::new(player.file_name.split('\\').last().unwrap_or(&player.file_name))
        .bold().centered()
        .block(Block::new().borders(Borders::ALL));
    frame.render_widget(file_name, layout[1]);
    // Debug //
    // let test = Paragraph::new(format!("~Debug~",)).centered();
    // frame.render_widget(test, layout[2]);
    // 操作方法 //
    let keybind = Paragraph::new(format!("~ 操作方法 ~
Esc / q / Ctrl + c : 終了
Space : 再生/一時停止
r : リプレイ
s : オーディオファイルの取り出し
← / → / Shift + ← / Shift + → : 再生位置の調整
↑ / ↓ / Shift + ↑ / Shift + ↓ : 音量の調整",)).centered();
    frame.render_widget(keybind, layout[2]);
    // 再生バー //
    let progress_percentage = if 0 < player.duration_total_time.as_secs() {
        ((player.duration_current_time.as_millis() as f64 / player.duration_total_time.as_millis() as f64) * 100.0) as u16
    } else { 0 };
    let progress_gauge = Gauge::default()
        .label(format!("{}:{:02}:{:02} / {}:{:02}:{:02}",
                        player.current_time.0,player.current_time.1,player.current_time.2,
                        player.total_time.0,player.total_time.1,player.total_time.2,
        ))
        .gauge_style(tailwind::BLUE.c500).percent(progress_percentage.clamp(0,100));
    frame.render_widget(progress_gauge, layout[3]);
    // 再生ボタン //
    let play_button = Line::from(if player.is_playing() { "▶ Resume ▶" } else { "- Pause -" })
        .centered();
    frame.render_widget(play_button, layout[4]);
    // 音量バー //
    let volume_gauge = Gauge::default()
        .label(format!("{}%", player.volume))
        .gauge_style(tailwind::GREEN.c500).percent(player.volume as u16)
        .block(Block::default().borders(Borders::ALL).title("音量"));
    frame.render_widget(volume_gauge, layout[5]);
}