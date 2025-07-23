use crate::interactive::interactiveplayer::InteractivePlayer;
use crate::interactive::ratatui::draw;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::DefaultTerminal;
use anyhow::Result;
use std::time::Duration;

pub struct App {
    pub player: InteractivePlayer,
    // files_list:Vec<String>, // 実装予定
    framerate: Duration,
    exit: bool,
}

impl Default for App {
    fn default() -> Self {
        Self {
            player: InteractivePlayer::new().unwrap(),
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

    pub fn run(&mut self, terminal: &mut DefaultTerminal, file_name: &str, volume: u8) -> Result<()> {
        self.player = InteractivePlayer::new()?;
        self.player.set_volume(volume);
        self.player.insert_and_play(&file_name)?;
        smol::block_on(async {
            while !self.exit {
                self.player.update_current_time();
                if let Err(e) = self.handle_events_async().await {
                    return Err(e);
                }
                terminal.draw(|frame| draw(frame, &self, &self.player))?;
                smol::Timer::after(self.framerate).await;
                if self.player.is_empty() && self.player.is_playing() {
                    self.exit = true;
                }
            }
            Ok(())
        })
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