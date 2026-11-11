//! OSのメディアコントロールとの連携
//!
//! Linux では MPRIS (D-Bus) にプレイヤーとして登録する。
//! Noctalia などのシェルはここからタイトルやジャケットを取得する。
//! Linux 以外では何もしない。
use std::time::Duration;

/// 外部（メディアキーやシェルのウィジェット）からの操作
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub enum Command {
    Play,
    Pause,
    PlayPause,
    Stop,
    Quit,
    /// 現在位置からの相対移動（マイクロ秒）
    SeekBy(i64),
    SeekTo(Duration),
    /// 音量 (1.0 = 100%)
    SetVolume(f64),
}

/// 外部に公開する再生状態
pub struct PlaybackState {
    pub playing: bool,
    pub stopped: bool,
    pub position: Duration,
    pub volume: u8,
    /// 前回の同期から再生位置が飛んだか
    pub seeked: bool,
}

pub use imp::MediaControl;

#[cfg(target_os = "linux")]
mod imp {
    use super::{Command, PlaybackState};
    use crate::core::track::{Cover, TrackInfo};
    use std::{env, fs, path::{Path, PathBuf}, process, time::Duration};
    use mpris_server::{Metadata, PlaybackStatus, Player, Time, TrackId};
    use smol::{channel::{unbounded, Receiver}, LocalExecutor};

    pub struct MediaControl {
        player: Option<Player>,
        commands: Receiver<Command>,
        cover_file: Option<PathBuf>,
    }

    impl Drop for MediaControl {
        fn drop(&mut self) {
            if let Some(cover_file) = &self.cover_file {
                _ = fs::remove_file(cover_file);
            }
        }
    }

    impl MediaControl {
        /// MPRIS に登録する。D-Bus が使えない環境では何もしないインスタンスを返す。
        pub async fn connect(executor:&LocalExecutor<'_>, track:&TrackInfo, file_path:&str, file_name:&str, length:Duration) -> Self {
            let (sender, commands) = unbounded();
            let cover_file = track.cover.as_ref().and_then(write_cover_file);

            let mut metadata = Metadata::builder()
                .title(track.title.as_deref().unwrap_or(file_name))
                .length(to_time(length));
            if let Ok(track_id) = TrackId::try_from("/io/github/c0b23092db/neiro/track/1") {
                metadata = metadata.trackid(track_id);
            }
            if let Some(artist) = &track.artist {
                metadata = metadata.artist([artist.as_str()]);
            }
            if let Some(album) = &track.album {
                metadata = metadata.album(album.as_str());
            }
            if let Ok(absolute_path) = fs::canonicalize(file_path) {
                metadata = metadata.url(file_url(&absolute_path));
            }
            if let Some(cover_file) = &cover_file {
                metadata = metadata.art_url(file_url(cover_file));
            }

            // 複数起動しても名前が衝突しないようにプロセスIDを付ける
            let player = Player::builder(&format!("neiro.instance{}", process::id()))
                .identity("Neiro")
                .can_play(true)
                .can_pause(true)
                .can_seek(true)
                .can_quit(true)
                .playback_status(PlaybackStatus::Playing)
                .metadata(metadata.build())
                .build()
                .await
                .ok();

            if let Some(player) = &player {
                // 操作はチャンネルに積み、メインループで処理する
                let send = move |command:Command| _ = sender.try_send(command);
                player.connect_play({ let send = send.clone(); move |_| send(Command::Play) });
                player.connect_pause({ let send = send.clone(); move |_| send(Command::Pause) });
                player.connect_play_pause({ let send = send.clone(); move |_| send(Command::PlayPause) });
                player.connect_stop({ let send = send.clone(); move |_| send(Command::Stop) });
                player.connect_quit({ let send = send.clone(); move |_| send(Command::Quit) });
                player.connect_seek({ let send = send.clone(); move |_, offset| send(Command::SeekBy(offset.as_micros())) });
                player.connect_set_position({ let send = send.clone(); move |_, _, position| {
                    send(Command::SeekTo(Duration::from_micros(position.as_micros().max(0) as u64)))
                }});
                player.connect_set_volume(move |_, volume| send(Command::SetVolume(volume)));
                executor.spawn(player.run()).detach();
            }

            Self { player, commands, cover_file }
        }

        pub fn poll_command(&self) -> Option<Command> {
            self.commands.try_recv().ok()
        }

        /// 再生状態を MPRIS に反映する。変化があったものだけ通知される。
        pub async fn sync(&mut self, state:&PlaybackState) {
            let Some(player) = &self.player else { return };
            let position = to_time(state.position);
            player.set_position(position);

            let status = if state.stopped { PlaybackStatus::Stopped }
                else if state.playing { PlaybackStatus::Playing }
                else { PlaybackStatus::Paused };
            if player.playback_status() != status {
                _ = player.set_playback_status(status).await;
            }
            let volume = state.volume as f64 / 100.0;
            if (player.volume() - volume).abs() > f64::EPSILON {
                _ = player.set_volume(volume).await;
            }
            if state.seeked {
                _ = player.seeked(position).await;
            }
        }
    }

    fn to_time(duration:Duration) -> Time {
        Time::from_micros(duration.as_micros().min(i64::MAX as u128) as i64)
    }

    /// シェルが読めるようにジャケットを一時ファイルに書き出す（終了時に削除）
    fn write_cover_file(cover:&Cover) -> Option<PathBuf> {
        let folder = env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from)
            .unwrap_or_else(env::temp_dir)
            .join("neiro");
        fs::create_dir_all(&folder).ok()?;
        let cover_file = folder.join(format!("cover-{}.{}", process::id(), cover.ext));
        fs::write(&cover_file, &cover.data).ok()?;
        Some(cover_file)
    }

    /// パスを file:// URL に変換する（パーセントエンコード）
    fn file_url(path:&Path) -> String {
        let mut url = String::from("file://");
        for byte in path.as_os_str().as_encoded_bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'/' | b'-' | b'_' | b'.' | b'~' => url.push(*byte as char),
                _ => url.push_str(&format!("%{:02X}", byte)),
            }
        }
        url
    }
}

#[cfg(not(target_os = "linux"))]
mod imp {
    use super::{Command, PlaybackState};
    use crate::core::track::TrackInfo;
    use std::time::Duration;
    use smol::LocalExecutor;

    pub struct MediaControl;

    impl MediaControl {
        pub async fn connect(_executor:&LocalExecutor<'_>, _track:&TrackInfo, _file_path:&str, _file_name:&str, _length:Duration) -> Self {
            Self
        }

        pub fn poll_command(&self) -> Option<Command> {
            None
        }

        pub async fn sync(&mut self, _state:&PlaybackState) {}
    }
}
