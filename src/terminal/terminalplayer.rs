use crate::core::{player, track};
use crate::interactive::mediacontrol::{Command, MediaControl, PlaybackState};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::Duration;
use anyhow::{Context, Result};

pub fn one_play(file_name:&str, volume:u8, timer:u64) -> Result<()> {
    let file_path = player::check_and_get_path(file_name)?;
    let (_stream_handle, sink) = player::initialize_soundplayer()?;
    let duration = player::append_one_track(&sink,file_path)?;
    let mut volume = volume.clamp(0,200);
    sink.set_volume(volume as f32 / 100.0);
    sink.play();
    let display_name = file_path.file_name().context("Invalid file name")?.to_string_lossy().into_owned();
    let (hours,minutes,seconds,millis) = player::format_duration(duration);
    print!("{} | {}:{:02}:{:02}:{:03} | ", display_name, hours, minutes, seconds, millis);
    // 再生を終了する位置（Noneは最後まで再生）
    let limit = if timer == 0 || duration.as_secs() < timer {
        println!("{}:{:02}:{:02}:{:03}",hours,minutes,seconds,millis);
        None
    }else{
        println!("{}:{:02}:{:02}",timer / 60 / 60, timer / 60 % 60, timer % 60);
        Some(Duration::from_secs(timer))
    };

    // Ctrl + C でも後始末（ジャケットの一時ファイルの削除）をしてから終了する
    let interrupted = Arc::new(AtomicBool::new(false));
    _ = ctrlc::set_handler({
        let interrupted = interrupted.clone();
        move || interrupted.store(true, Ordering::Relaxed)
    });

    let track = track::read_track_info(file_path);
    let executor = smol::LocalExecutor::new();
    smol::block_on(executor.run(async {
        let mut media = MediaControl::connect(&executor, &track, file_name, &display_name, duration).await;
        let mut seeked = false;
        while !interrupted.load(Ordering::Relaxed) && !sink.empty() {
            if limit.is_some_and(|limit| sink.get_pos() >= limit) {
                break;
            }
            // メディアコントロール（MPRISなど）からの操作 //
            while let Some(command) = media.poll_command() {
                let seek_to = match command {
                    Command::Play => { sink.play(); None },
                    Command::Pause => { sink.pause(); None },
                    Command::PlayPause => { if sink.is_paused() { sink.play() } else { sink.pause() }; None },
                    Command::Stop | Command::Quit => { sink.stop(); None },
                    Command::SeekBy(micros) => {
                        let offset = Duration::from_micros(micros.unsigned_abs());
                        Some(if micros.is_negative() { sink.get_pos().saturating_sub(offset) } else { sink.get_pos() + offset })
                    },
                    Command::SeekTo(position) => Some(position),
                    Command::SetVolume(new_volume) => {
                        volume = (new_volume.clamp(0.0, 2.0) * 100.0).round() as u8;
                        sink.set_volume(volume as f32 / 100.0);
                        None
                    },
                };
                // シークできない形式でも再生は続ける
                if let Some(position) = seek_to && sink.try_seek(position.min(duration)).is_ok() {
                    seeked = true;
                }
            }
            media.sync(&PlaybackState {
                playing: !sink.is_paused(),
                stopped: sink.empty(),
                position: sink.get_pos(),
                volume,
                seeked: std::mem::take(&mut seeked),
            }).await;
            smol::Timer::after(Duration::from_millis(50)).await;
        }
    }));
    Ok(())
}
