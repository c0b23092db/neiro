use crate::core::player;
use std::{thread, time::Duration};
use anyhow::Result;

pub fn one_play(file_name:&str, volume:u8, mut timer:u64) -> Result<()> {
    let file_path = player::check_and_get_path(file_name)?;
    let (_stream_handle, sink) = player::initialize_soundplayer()?;
    let duration = player::append_one_track(&sink,file_path)?;
    let (hours,minutes,seconds,_) = player::format_duration(duration);
    sink.set_volume(volume.clamp(0,100) as f32 / 100.0);
    if timer == 0 || duration.as_secs() < timer {
        timer = duration.as_secs();
    }
    println!("{} | {}:{:02}:{:02} | {}:{:02}:{:02}",
                file_path.file_name().unwrap().to_string_lossy().into_owned(), hours, minutes, seconds,
                timer / 60 / 60, timer / 60 % 60, timer % 60);
    sink.play();
    thread::sleep(Duration::from_secs(timer));
    Ok(())
}