use crate::core::player;
use std::{thread, time::Duration};
use anyhow::Result;

pub fn one_play(file_name:&str, volume:u8, timer:u64) -> Result<()> {
    let file_path = player::check_and_get_path(file_name)?;
    let (_stream_handle, sink) = player::initialize_soundplayer()?;
    let mut duration = player::append_one_track(&sink,file_path)?;
    sink.set_volume(volume.clamp(0,200) as f32 / 100.0);
    sink.play();
    let (hours,minutes,seconds,millis) = player::format_duration(duration);
    print!("{} | {}:{:02}:{:02}:{:03} | ",
                file_path.file_name()?.to_string_lossy().into_owned(),
                hours, minutes, seconds, millis);
    if timer == 0 || duration.as_secs() < timer {
        println!("{}:{:02}:{:02}:{:03}",hours,minutes,seconds,millis);
    }else{
        duration = Duration::from_secs(timer);
        println!("{}:{:02}:{:02}",timer / 60 / 60, timer / 60 % 60, timer % 60);
    }
    thread::sleep(duration);
    Ok(())
}