use std::{fs::File,path::Path,time::Duration};
use rodio::{Decoder, OutputStream, OutputStreamBuilder, Sink, Source};
use anyhow::{Context,Result,anyhow,bail};

pub fn check_and_get_path(file_path:&str) -> Result<&Path> {
    let path = Path::new(file_path);
    if !path.is_file() {
        bail!("Audio file not found：{}", path.display());
    }
    Ok(path)
}

pub fn initialize_soundplayer() -> Result<(OutputStream,Sink)> {
    let mut stream_handle = OutputStreamBuilder::open_default_stream()
        .map_err(|e| anyhow!("Failed to initialize OutputStream in rodio：{}",e))?;
    stream_handle.log_on_drop(false);
    let sink = Sink::connect_new(&stream_handle.mixer());
    Ok((stream_handle,sink))
}

/// ## Description
/// Sinkに音声ファイルを追加
/// ## Arguments
/// sink:Sinkのインスタンス, path:音声ファイルのパス
/// ## Returns
/// hours:時間,minutes:分,seconds:秒,millis:ミリ秒
pub fn append_one_track(sink:&Sink,path:&Path) -> Result<Duration> {
    let file_data = File::open(path)
        .with_context(|| format!("Failed to open file：{}", path.display()))?;
    let decoder = Decoder::try_from(file_data)
        .map_err(|e| anyhow!("Failed to create decoder in rodio：{}",e))?;
    let duration = decoder.total_duration().unwrap_or(Duration::ZERO);
    sink.append(decoder);
    Ok(duration)
}

/// ## Description
/// Durationの秒数をDuration、時間、分、秒、ミリ秒に変換
pub fn format_duration(duration:Duration) -> (u64,u64,u64,u32) {
    let hours = duration.as_secs() / 60 / 60;
    let minutes = duration.as_secs() / 60;
    let seconds = duration.as_secs() % 60;
    let millis = duration.subsec_millis();
    return (hours,minutes,seconds,millis);
}