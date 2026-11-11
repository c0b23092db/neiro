use crate::interactive::interactiveplayer::InteractivePlayer;
use crate::interactive::mediacontrol::{Command, MediaControl, PlaybackState};
use crate::interactive::ratatui::draw;
use crate::core::track::{self, TrackInfo};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use ratatui_image::{picker::Picker, protocol::StatefulProtocol};
use anyhow::Result;
use std::{path::Path, time::Duration};

pub struct App {
    pub player: InteractivePlayer,
    pub track: TrackInfo,
    /// ターミナルに表示するジャケット
    pub cover: Option<StatefulProtocol>,
    /// 再生位置が飛んだことをメディアコントロールに伝えるフラグ
    seeked: bool,
    /// ヘルプ（操作方法）を表示しているか
    show_help: bool,
    // files_list:Vec<String>, // 実装予定
    framerate: Duration,
    exit: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            player: InteractivePlayer::new().unwrap(),
            track: TrackInfo::default(),
            cover: None,
            seeked: false,
            show_help: false,
            // files_list:Vec::new(),
            framerate: Duration::from_millis(16), // 62.5HZ / 62.5FPS
            exit: false,
        }
    }
}

impl App {
    pub fn new() -> Self {
        return App::default();
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal, file_name: &str, volume: u8, timer: u64) -> Result<()> {
        self.player = InteractivePlayer::new()?;
        self.player.set_volume(volume);
        self.player.insert_and_play(&file_name)?;
        self.track = track::read_track_info(Path::new(file_name));
        self.cover = self.track.cover.as_ref().and_then(|cover| load_cover(&cover.data));
        // メディアコントロールのタスクを同じスレッドで動かす
        let executor = smol::LocalExecutor::new();
        smol::block_on(executor.run(async {
            let mut media = MediaControl::connect(
                &executor, &self.track, &self.player.file_path, &self.player.file_name, self.player.duration_total_time,
            ).await;
            while !self.exit {
                self.player.update_current_time();
                if let Err(e) = self.handle_events_async().await {
                    return Err(e);
                }
                while let Some(command) = media.poll_command() {
                    self.handle_command(command)?;
                }
                terminal.draw(|frame| draw(frame, &self.player, &self.track, self.cover.as_mut(), self.show_help))?;
                media.sync(&self.playback_state()).await;
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

    /// メディアコントロール（MPRISなど）からの操作を処理する
    fn handle_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::Play => self.player.resume(),
            Command::Pause => self.player.pause(),
            Command::PlayPause => _ = self.player.switch_playback(),
            Command::Stop | Command::Quit => self.exit(),
            Command::SeekBy(micros) => {
                let offset = Duration::from_micros(micros.unsigned_abs());
                let current = self.player.duration_current_time;
                let position = if micros.is_negative() { current.saturating_sub(offset) } else { current + offset };
                self.seek_to(position)?
            },
            Command::SeekTo(position) => self.seek_to(position)?,
            Command::SetVolume(volume) => self.player.set_volume((volume.clamp(0.0, 1.0) * 100.0).round() as u8),
        }
        Ok(())
    }

    fn seek_to(&mut self, position: Duration) -> Result<()> {
        self.player.seek_to(position)?;
        self.after_seek();
        Ok(())
    }

    fn after_seek(&mut self) {
        self.player.get_current_position();
        self.seeked = true;
    }

    fn playback_state(&mut self) -> PlaybackState {
        PlaybackState {
            playing: self.player.is_playing(),
            stopped: self.player.is_empty(),
            position: self.player.duration_current_time,
            volume: self.player.volume,
            seeked: std::mem::take(&mut self.seeked),
        }
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
                let plus_or_minus = matches!(key, KeyCode::Up);
                let volume = if modifier == KeyModifiers::SHIFT { 10 } else { 5 };
                self.set_volume(volume, plus_or_minus)?
            },
            // 再生位置を変更 //
            (modifier, key @ (KeyCode::Left | KeyCode::Right)) => {
                let mut secs = if modifier == KeyModifiers::SHIFT { 10 } else { 5 };
                secs *= if matches!(key, KeyCode::Right) { 1 } else { -1 };
                self.player.skep_seek(secs)?;
                self.after_seek()
            },
            // 参考コード // 5,10秒進む //
            // (KeyModifiers::SHIFT, KeyCode::Right) => self.skep_seek(10)?,
            // (_, KeyCode::Right) => self.skep_seek(5)?,
            // リプレイ //
            (_, KeyCode::Char('r')) => {
                self.player.jump_seek(u64::MIN)?;
                self.after_seek()
            },
            // 取り出し //
            (_, KeyCode::Char('s')) => self.player.stop(),
            _ => {}

        }
        Ok(())
    }

    fn set_volume(&mut self, volume:u8, plus_or_minus:bool) -> Result<()> {
        if plus_or_minus && 1.0 != self.player.get_volume(){
            self.player.set_volume((self.player.get_volume() * 100.0) as u8 + volume);
        }else if !plus_or_minus && 0.0 != self.player.get_volume(){
            self.player.set_volume((self.player.get_volume() * 100.0) as u8 - volume);
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
