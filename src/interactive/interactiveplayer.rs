use crate::core::player;
use std::time::Duration;
use rodio::{OutputStream, Sink};
use anyhow::{Result, anyhow};
use smol::channel::{unbounded, Receiver, Sender};
use std::sync::{Arc, Mutex};

pub struct InteractivePlayer {
    pub sink: Sink,
    _stream_handle: OutputStream,
    pub file_name: String,
    pub volume: u8,
    pub duration_total_time: Duration,
    pub total_time: (u64, u64, u64, u32),
    pub duration_current_time: Duration,
    pub current_time: (u64, u64, u64, u32),
    pub time_update_sender: Sender<()>,
    pub time_update_receiver: Receiver<()>,
    pub time_update_stop: Arc<Mutex<bool>>,
}

impl InteractivePlayer {
    pub fn new() -> Result<Self> {
        let (_stream_handle, sink) = player::initialize_soundplayer()?;
        let (time_update_sender, time_update_receiver) = unbounded();
        let time_update_stop = Arc::new(Mutex::new(false));
        Ok(Self {
            sink,
            _stream_handle,
            file_name: "".to_string(),
            volume: 50,
            duration_total_time: Duration::from_secs(0),
            total_time: player::format_duration(Duration::from_secs(0)),
            duration_current_time: Duration::from_secs(0),
            current_time: player::format_duration(Duration::from_secs(0)),
            time_update_sender,
            time_update_receiver,
            time_update_stop,
        })
    }

    pub fn insert_and_play(&mut self, file_name: &str) -> Result<()> {
        self.insert(file_name)?;
        self.sink.play();
        Ok(())
    }

    pub fn insert(&mut self, file_name: &str) -> Result<()> {
        self.file_name = file_name.to_string();
        let path = player::check_and_get_path(&self.file_name)?;
        self.duration_total_time = player::append_one_track(&self.sink, path)?;
        self.total_time = player::format_duration(self.duration_total_time);
        self.start_time_update_task();
        Ok(())
    }

    pub fn pause(&self) {
        self.sink.pause();
    }

    pub fn resume(&self) {
        self.sink.play();
    }

    pub fn switch_playback(&self) -> bool {
        if self.sink.is_paused() {
            self.resume();
            return true;
        } else {
            self.pause();
            return false;
        }
    }

    pub fn stop(&self) {
        self.sink.stop();
        if let Ok(mut stop_flag) = self.time_update_stop.lock() {
            *stop_flag = true;
        }
    }

    pub fn seek(&self, secs: Duration) -> Result<()> {
        self.sink
            .try_seek(secs)
            .map_err(|e| anyhow!("Failed to seek in rodio：{}", e))?;
        Ok(())
    }

    pub fn set_volume(&mut self, volume: u8) {
        self.volume = volume;
        self.sink.set_volume(volume as f32 / 100.0);
    }

    pub fn update_current_position(&mut self) {
        self.duration_current_time = self.sink.get_pos();
        self.current_time = player::format_duration(self.duration_current_time);
    }

    pub fn get_volume(&self) -> f32 {
        return self.sink.volume();
    }

    pub fn is_playing(&self) -> bool {
        return !self.sink.is_paused();
    }

    pub fn is_empty(&self) -> bool {
        return self.sink.empty();
    }

    fn start_time_update_task(&self) {
        let sender = self.time_update_sender.clone();
        let stop_update = self.time_update_stop.clone();
        smol::spawn(async move {
            loop {
                if let Ok(stop_flag) = stop_update.lock() {
                    if *stop_flag {
                        break;
                    }
                }
                if sender.send(()).await.is_err() {
                    break;
                }
                smol::Timer::after(Duration::from_millis(100)).await;
            }
        }).detach();
    }

    pub fn check_time_update(&mut self) -> bool {
        if let Ok(()) = self.time_update_receiver.try_recv() {
            self.update_current_position();
            true
        } else {
            false
        }
    }
}