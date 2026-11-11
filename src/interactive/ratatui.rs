use crate::core::track::TrackInfo;
use crate::interactive::interactiveplayer::InteractivePlayer;
use ratatui::{
    style::{palette::tailwind, Stylize},
    text::{Line},
    widgets::{Block, Paragraph, Gauge},
    layout::Flex,
    prelude::{Layout, Constraint, Direction, Rect},
    Frame,
};
use ratatui_image::{protocol::StatefulProtocol, Resize, StatefulImage};

const HELP: &str = "~ 操作方法 ~
Esc / q / Ctrl + c : 終了
Space : 再生/一時停止
r : リプレイ
s : オーディオファイルの取り出し
← / → / Shift + ← / Shift + → : 再生位置の調整
↑ / ↓ / Shift + ↑ / Shift + ↓ : 音量の調整
? : 操作方法の表示/非表示";

pub fn draw(frame: &mut Frame, player: &InteractivePlayer, track: &TrackInfo, cover: Option<&mut StatefulProtocol>, show_help: bool) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![
            Constraint::Length(3), // 実行ファイル名・ファイル名
            Constraint::Percentage(100),
            Constraint::Length(1), // 再生バー
            Constraint::Length(1), // 再生ボタン
            Constraint::Length(3), // 音量バー
        ])
        .split(frame.area());

    // 実行ファイル名・ファイル名 //
    let file_name = Paragraph::new(&*player.file_name)
        .white().centered()
        .block(Block::bordered().title("Neiro").bold().blue());
    frame.render_widget(file_name, layout[0]);
    // Debug //
    // let test = Paragraph::new(format!("~Debug~",)).centered();
    // frame.render_widget(test, layout[2]);
    if show_help {
        // 操作方法 //
        // 画像の上に文字を重ねられないターミナルがあるので、ジャケットと切り替えて表示する
        let [help_area] = Layout::vertical([Constraint::Length(HELP.lines().count() as u16)])
            .flex(Flex::Center).areas(layout[1]);
        frame.render_widget(Paragraph::new(HELP).centered(), help_area);
    } else {
        draw_track(frame, layout[1], track, cover);
    }
    // 再生バー //
    let progress_percentage = ((player.duration_current_time.as_millis() as f64 / player.duration_total_time.as_millis() as f64) * 100.0) as u16;
    let progress_gauge = Gauge::default()
        .label(format!("{}:{:02}:{:02}:{:03} / {}:{:02}:{:02}:{:03}",
                        player.current_time.0,player.current_time.1,player.current_time.2,player.current_time.3,
                        player.total_time.0,player.total_time.1,player.total_time.2, player.total_time.3,
        ))
        .gauge_style(tailwind::BLUE.c500).percent(progress_percentage);
    frame.render_widget(progress_gauge, layout[2]);
    // 再生ボタン //
    let play_button = Line::from(if player.is_playing() { "▶ Resume ▶" } else { "- Pause -" })
        .centered();
    frame.render_widget(play_button, layout[3]);
    // 音量バー //
    let volume_gauge = Gauge::default()
        .block(Block::bordered().title("音量").title_bottom(Line::from("? : 操作方法").right_aligned()))
        .label(format!("{}%", player.volume))
        .gauge_style(tailwind::GREEN.c500).percent(player.volume as u16);
    frame.render_widget(volume_gauge, layout[4]);
}

/// ジャケットとタグ情報を中央に表示する
fn draw_track(frame: &mut Frame, area: Rect, track: &TrackInfo, cover: Option<&mut StatefulProtocol>) {
    let tags:Vec<Line> = [("Title", &track.title), ("Artist", &track.artist), ("Album", &track.album)]
        .into_iter()
        .filter_map(|(label, value)| Some(Line::from(format!("{} : {}", label, value.as_ref()?))))
        .collect();
    let tags_height = tags.len() as u16;
    // 画面が狭いときはタグ情報を優先してジャケットを表示しない
    let cover = cover.filter(|_| area.height >= tags_height + 4);
    let Some(cover) = cover else {
        let [tags_area] = Layout::vertical([Constraint::Length(tags_height)]).flex(Flex::Center).areas(area);
        frame.render_widget(Paragraph::new(tags).centered(), tags_area);
        return;
    };
    let [cover_area, tags_area] = Layout::vertical([Constraint::Min(0), Constraint::Length(tags_height)]).areas(area);
    let size = cover.size_for(Resize::Fit(None), cover_area.as_size());
    let centered = Rect::new(
        cover_area.x + cover_area.width.saturating_sub(size.width) / 2,
        cover_area.y + cover_area.height.saturating_sub(size.height) / 2,
        size.width.min(cover_area.width),
        size.height.min(cover_area.height),
    );
    frame.render_stateful_widget(StatefulImage::default(), centered, cover);
    frame.render_widget(Paragraph::new(tags).centered(), tags_area);
}
