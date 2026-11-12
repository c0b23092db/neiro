use std::{env, path::Path, process::{Command, Stdio}};
use anyhow::{Context, Result, bail};

/// ## Description
/// CLIモードの自分自身を端末から切り離して起動し、すぐに戻る
pub fn play_in_background(file_name:&str, volume:u8, timer:u64, repeat:bool) -> Result<()> {
    // 起動後のエラーは表示できないので、ファイルの有無は先に確認する
    let path = Path::new(file_name);
    if !path.is_file() {
        bail!("Audio file not found：{}", path.display());
    }
    let mut command = Command::new(env::current_exe().context("Failed to get the path of the executable")?);
    command.arg("--cli")
        .args(["--volume", &volume.to_string(), "--timer", &timer.to_string()])
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    if repeat {
        command.arg("--repeat");
    }
    // ファイル名が - で始まっていてもオプションとして解釈されないようにする
    command.arg("--").arg(file_name);
    detach(&mut command);
    let child = command.spawn().context("Failed to start playback in the background")?;
    println!("{} | PID {}", path.file_name().unwrap_or(path.as_os_str()).to_string_lossy(), child.id());
    Ok(())
}

/// 端末の Ctrl + C などが届かないように、別のプロセスグループで起動する
#[cfg(unix)]
fn detach(command:&mut Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(windows)]
fn detach(command:&mut Command) {
    use std::os::windows::process::CommandExt;
    const DETACHED_PROCESS: u32 = 0x00000008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x00000200;
    command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
}

#[cfg(not(any(unix, windows)))]
fn detach(_command:&mut Command) {}
