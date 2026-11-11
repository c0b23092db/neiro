use crate::core::mediacontrol::{Command, PlaybackState};
use std::{fs::File,path::Path,time::Duration};
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player, Source};
use anyhow::{Context,Result,anyhow,bail};

/// 指定できる音量の上限（%）
pub const MAX_VOLUME: u8 = 200;

/// TUIモード・CLIモードで共通のオーディオプレイヤー
pub struct AudioPlayer {
    player: Player,
    _stream_handle: MixerDeviceSink,
    pub file_path: String,
    pub file_name: String,
    pub volume: u8,
    pub duration_total_time: Duration,
    pub total_time: (u64, u64, u64, u32),
    pub duration_current_time: Duration,
    pub current_time: (u64, u64, u64, u32),
    /// 前回の playback_state() から再生位置が飛んだか
    seeked: bool,
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.player.stop();
    }
}

impl AudioPlayer {
    pub fn new() -> Result<Self> {
        let (_stream_handle, player) = initialize_soundplayer()?;
        Ok(Self {
            player,
            _stream_handle,
            file_path: "".to_string(),
            file_name: "".to_string(),
            volume: 50,
            duration_total_time: Duration::ZERO,
            total_time: format_duration(Duration::ZERO),
            duration_current_time: Duration::ZERO,
            current_time: format_duration(Duration::ZERO),
            seeked: false,
        })
    }

    pub fn insert_and_play(&mut self, file_name: &str) -> Result<()> {
        self.insert(file_name)?;
        self.player.play();
        Ok(())
    }

    pub fn insert(&mut self, file_name: &str) -> Result<()> {
        let path = check_and_get_path(file_name)?;
        self.file_path = file_name.to_string();
        self.file_name = path.file_name().context("Invalid file name")?.to_string_lossy().into_owned();
        self.duration_total_time = append_one_track(&self.player, path)?;
        self.total_time = format_duration(self.duration_total_time);
        Ok(())
    }

    pub fn pause(&self) {
        self.player.pause();
    }

    pub fn resume(&self) {
        self.player.play();
    }

    pub fn switch_playback(&self) -> bool {
        if self.player.is_paused() {
            self.resume();
            return true;
        } else {
            self.pause();
            return false;
        }
    }

    pub fn stop(&self) {
        self.player.stop();
    }

    pub fn jump_seek(&mut self, secs:u64) -> Result<()> {
        self.seek_to(Duration::from_secs(secs))
    }

    pub fn skep_seek(&mut self, secs:i64) -> Result<()> {
        self.seek_by(Duration::from_secs(secs.unsigned_abs()), secs.is_negative())
    }

    /// 現在位置から相対的に移動する
    fn seek_by(&mut self, offset:Duration, backward:bool) -> Result<()> {
        let current = self.player.get_pos();
        self.seek_to(if backward { current.saturating_sub(offset) } else { current + offset })
    }

    /// 指定した位置に移動する（曲の長さを超える場合は末尾に丸める）
    pub fn seek_to(&mut self, position:Duration) -> Result<()> {
        // 長さが取得できなかった曲は丸めない
        let position = if self.duration_total_time.is_zero() { position } else { position.min(self.duration_total_time) };
        self.player.try_seek(position)
            .map_err(|e| anyhow!("Failed to seek in rodio: {}", e))?;
        self.get_current_position();
        self.seeked = true;
        Ok(())
    }

    /// 音量を設定する（0 ~ MAX_VOLUME に丸める）
    pub fn set_volume(&mut self, volume: u8) {
        self.volume = volume.min(MAX_VOLUME);
        self.player.set_volume(self.volume as f32 / 100.0);
    }

    /// 音量を増減する
    pub fn change_volume(&mut self, delta: i8) {
        self.set_volume(self.volume.saturating_add_signed(delta));
    }

    pub fn get_current_position(&mut self) {
        self.duration_current_time = self.player.get_pos();
        self.current_time = format_duration(self.duration_current_time);
    }

    pub fn is_playing(&self) -> bool {
        return !self.player.is_paused();
    }

    pub fn is_empty(&self) -> bool {
        return self.player.empty();
    }

    /// メディアコントロール（MPRISなど）からの操作を反映する
    ///
    /// 外部からの操作でプレイヤーを落とさないよう、シークの失敗は無視する。
    pub fn apply(&mut self, command: Command) {
        match command {
            Command::Play => self.resume(),
            Command::Pause => self.pause(),
            Command::PlayPause => _ = self.switch_playback(),
            Command::Stop | Command::Quit => self.stop(),
            Command::SeekBy(micros) => _ = self.seek_by(Duration::from_micros(micros.unsigned_abs()), micros.is_negative()),
            Command::SeekTo(position) => _ = self.seek_to(position),
            Command::SetVolume(volume) => self.set_volume((volume.clamp(0.0, MAX_VOLUME as f64 / 100.0) * 100.0).round() as u8),
        }
    }

    /// メディアコントロールに公開する再生状態を取得する
    pub fn playback_state(&mut self) -> PlaybackState {
        PlaybackState {
            playing: self.is_playing(),
            stopped: self.is_empty(),
            position: self.duration_current_time,
            volume: self.volume,
            seeked: std::mem::take(&mut self.seeked),
        }
    }
}

fn check_and_get_path(file_path:&str) -> Result<&Path> {
    let path = Path::new(file_path);
    if !path.is_file() {
        bail!("Audio file not found：{}", path.display());
    }
    Ok(path)
}

fn initialize_soundplayer() -> Result<(MixerDeviceSink,Player)> {
    let mut stream_handle = DeviceSinkBuilder::open_default_sink()
        .map_err(|e| anyhow!("Failed to initialize MixerDeviceSink in rodio：{}",e))?;
    stream_handle.log_on_drop(false);
    let player = Player::connect_new(&stream_handle.mixer());
    Ok((stream_handle,player))
}

/// ## Description
/// Playerに音声ファイルを追加
/// ## Arguments
/// player:Playerのインスタンス, path:音声ファイルのパス
/// ## Returns
/// 曲の長さ（取得できない場合は0）
fn append_one_track(player:&Player,path:&Path) -> Result<Duration> {
    let file_data = File::open(path)
        .with_context(|| format!("Failed to open file：{}", path.display()))?;
    let decoder = Decoder::try_from(file_data)
        .map_err(|e| anyhow!("Failed to create decoder in rodio：{}",e))?;
    let duration = decoder.total_duration().unwrap_or(Duration::ZERO);
    player.append(decoder);
    Ok(duration)
}

/// ## Description
/// Durationを時間、分、秒、ミリ秒に変換
pub fn format_duration(duration:Duration) -> (u64,u64,u64,u32) {
    let hours = duration.as_secs() / 60 / 60;
    let minutes = duration.as_secs() / 60 % 60;
    let seconds = duration.as_secs() % 60;
    let millis = duration.subsec_millis();
    return (hours,minutes,seconds,millis);
}
