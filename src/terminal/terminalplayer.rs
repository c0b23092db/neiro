use crate::core::audio::AudioPlayer;
use crate::core::mediacontrol::MediaControl;
use crate::core::track;
use std::path::Path;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}};
use std::time::Duration;
use anyhow::Result;

pub fn one_play(file_name:&str, volume:u8, timer:u64) -> Result<()> {
    let mut player = AudioPlayer::new()?;
    player.set_volume(volume);
    player.insert_and_play(file_name)?;
    let duration = player.duration_total_time;
    let (hours,minutes,seconds,millis) = player.total_time;
    print!("{} | {}:{:02}:{:02}:{:03} | ", player.file_name, hours, minutes, seconds, millis);
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

    let track = track::read_track_info(Path::new(file_name));
    // メディアコントロールのタスクを同じスレッドで動かす
    let executor = smol::LocalExecutor::new();
    smol::block_on(executor.run(async {
        let mut media = MediaControl::connect(&executor, &track, &player.file_path, &player.file_name, duration).await;
        while !interrupted.load(Ordering::Relaxed) && !player.is_empty() {
            player.get_current_position();
            if limit.is_some_and(|limit| player.duration_current_time >= limit) {
                break;
            }
            while let Some(command) = media.poll_command() {
                player.apply(command);
            }
            media.sync(&player.playback_state()).await;
            smol::Timer::after(Duration::from_millis(50)).await;
        }
    }));
    Ok(())
}
