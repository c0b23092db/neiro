use crate::core::audio::AudioPlayer;
use crate::core::mediacontrol::{Command, MediaControl};
use crate::core::track::{self, TrackInfo};
use crate::interactive::ratatui::draw;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use ratatui_image::{picker::Picker, protocol::StatefulProtocol};
use anyhow::Result;
use std::{path::Path, time::Duration};

pub struct App {
    pub player: AudioPlayer,
    pub track: TrackInfo,
    /// ターミナルに表示するジャケット
    pub cover: Option<StatefulProtocol>,
    /// ヘルプ（操作方法）を表示しているか
    show_help: bool,
    // files_list:Vec<String>, // 実装予定
    framerate: Duration,
    exit: bool,
}

impl App {
    /// オーディオファイルを読み込み、再生を開始する
    pub fn new(file_name: &str, volume: u8) -> Result<Self> {
        let mut player = AudioPlayer::new()?;
        player.set_volume(volume);
        player.insert_and_play(file_name)?;
        Ok(Self {
            player,
            track: track::read_track_info(Path::new(file_name)),
            cover: None,
            show_help: false,
            // files_list:Vec::new(),
            framerate: Duration::from_millis(16), // 62.5HZ / 62.5FPS
            exit: false,
        })
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal, timer: u64) -> Result<()> {
        // ターミナルへの問い合わせが必要なので、画面の初期化後に変換する
        self.cover = self.track.cover.as_ref().and_then(|cover| load_cover(&cover.data));
        // メディアコントロールのタスクを同じスレッドで動かす
        let executor = smol::LocalExecutor::new();
        smol::block_on(executor.run(async {
            let mut media = MediaControl::connect(
                &executor, &self.track, &self.player.file_path, &self.player.file_name, self.player.duration_total_time,
            ).await;
            while !self.exit {
                self.player.get_current_position();
                if let Err(e) = self.handle_events_async().await {
                    return Err(e);
                }
                // メディアコントロール（MPRISなど）からの操作 //
                while let Some(command) = media.poll_command() {
                    match command {
                        Command::Stop | Command::Quit => self.exit(),
                        command => self.player.apply(command),
                    }
                }
                terminal.draw(|frame| draw(frame, &self.player, &self.track, self.cover.as_mut(), self.show_help))?;
                media.sync(&self.player.playback_state()).await;
                smol::Timer::after(self.framerate).await;
                if self.player.is_empty() && self.player.is_playing() {
                    self.exit = true;
                }
                // 再生時間の指定がある場合はそこで終了する
                if timer != 0 && self.player.duration_current_time >= Duration::from_secs(timer) {
                    self.exit();
                }
            }
            Ok(())
        }))
    }

    async fn handle_events_async(&mut self) -> Result<()> {
        if event::poll(Duration::ZERO)? {
            match event::read()? {
                Event::Key(input) if input.kind == KeyEventKind::Press => {
                    self.handle_key_event(input)?;
                }
                Event::Mouse(_) => {}
                Event::Resize(_, _) => {}
                _ => {}
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) -> Result<()> {
        match (key_event.modifiers, key_event.code) {
            // ヘルプの表示 / 非表示 //
            (_, KeyCode::Char('?')) => self.show_help = !self.show_help,
            (_, KeyCode::Esc) if self.show_help => self.show_help = false,
            // 終了 //
            (_, KeyCode::Esc | KeyCode::Char('q'))
            | (KeyModifiers::CONTROL, KeyCode::Char('c') | KeyCode::Char('C')) => self.exit(),
            // 再生 / 一時停止 //
            (_, KeyCode::Char(' ')) => _ = self.player.switch_playback(),
            // 音量を変更 //
            (modifier, key @ (KeyCode::Up | KeyCode::Down)) => {
                let mut volume = if modifier == KeyModifiers::SHIFT { 10 } else { 5 };
                volume *= if matches!(key, KeyCode::Up) { 1 } else { -1 };
                self.player.change_volume(volume)
            },
            // 再生位置を変更 //
            (modifier, key @ (KeyCode::Left | KeyCode::Right)) => {
                let mut secs = if modifier == KeyModifiers::SHIFT { 10 } else { 5 };
                secs *= if matches!(key, KeyCode::Right) { 1 } else { -1 };
                self.player.skep_seek(secs)?
            },
            // 参考コード // 5,10秒進む //
            // (KeyModifiers::SHIFT, KeyCode::Right) => self.skep_seek(10)?,
            // (_, KeyCode::Right) => self.skep_seek(5)?,
            // リプレイ //
            (_, KeyCode::Char('r')) => self.player.jump_seek(u64::MIN)?,
            // 取り出し //
            (_, KeyCode::Char('s')) => self.player.stop(),
            _ => {}

        }
        Ok(())
    }

    fn exit(&mut self) {
        self.exit = true;
        self.player.stop();
    }
}

/// ジャケットをターミナル表示用に変換する
///
/// Kitty / Sixel / iTerm2 の画像プロトコルに対応していればそれを使い、
/// 非対応のターミナルでは半角ブロック文字で描画する。
fn load_cover(data: &[u8]) -> Option<StatefulProtocol> {
    const MAX_SIZE: u32 = 1024;
    let mut image = image::load_from_memory(data).ok()?;
    if image.width().max(image.height()) > MAX_SIZE {
        image = image.thumbnail(MAX_SIZE, MAX_SIZE);
    }
    let picker = Picker::from_query_stdio().unwrap_or_else(|_| Picker::halfblocks());
    Some(picker.new_resize_protocol(image))
}
