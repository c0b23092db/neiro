mod core;
mod terminal;
mod interactive;
mod background;
use std::io::IsTerminal;
use std::process::ExitCode;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use background::control::Action;

#[derive(Debug,Parser)]
#[command(author,version,about,arg_required_else_help = true)]
#[clap(
long_about = "Neiro / Simple Audio File Player",
// 操作コマンド（pause など）のときはファイル名を要求しない
args_conflicts_with_subcommands = true,
)]
struct Cli {
    #[command(subcommand)]
    command:Option<Commands>,
    #[clap(
        help = "Audio file path",
        value_name = "FILE",
    )]
    file_name:Option<String>,
    #[clap(
        help = "Timer of Audio file (0 for full playback)",
        long = "timer",
        short = 't',
        default_value_t = 0,
    )]
    timer:u64,
    #[clap(
        help = "Volume of Audio file [0-200]",
        long = "volume",
        short = 'v',
        default_value_t = 50,
    )]
    volume:u8,
    #[clap(
        help = "Repeat the Audio file",
        long = "repeat",
        short = 'r',
    )]
    repeat:bool,
    #[clap(
        help = "Run in TUI mode with UI controls (default)",
        long = "tui",
        alias = "interactive",
        short_alias = 'i',
        conflicts_with_all = ["mode_cli", "mode_async"],
    )]
    mode_tui:bool,
    #[clap(
        help = "Run in CLI mode without UI",
        long = "cli",
        short = 'c',
        conflicts_with = "mode_async",
    )]
    mode_cli:bool,
    #[clap(
        help = "Play in the background and return immediately",
        long = "async",
        short = 'a',
    )]
    mode_async:bool,
}

#[derive(Debug,Subcommand)]
enum Commands {
    #[command(about = "Pause the Neiro playing in the background")]
    Pause,
    #[command(about = "Resume the Neiro playing in the background")]
    Resume,
    #[command(about = "Stop the Neiro playing in the background")]
    Stop,
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}",e);
            ExitCode::FAILURE
        }
    }
}

fn run(args:Cli) -> Result<()> {
    if let Some(command) = args.command {
        let action = match command {
            Commands::Pause => Action::Pause,
            Commands::Resume => Action::Resume,
            Commands::Stop => Action::Stop,
        };
        return background::control::send(action).map(|_| ());
    }
    let file_name = args.file_name.as_deref().context("Audio file path is required")?;
    if args.mode_async {
        background::spawn::play_in_background(file_name, args.volume, args.timer, args.repeat)
    }
    // 標準はTUIモード。パイプやリダイレクトなど端末でない場合はCLIモードで再生する
    else if args.mode_tui || (!args.mode_cli && std::io::stdout().is_terminal()) {
        interactive::run::run_interactive_player(file_name, args.volume, args.timer, args.repeat)
    }
    else { terminal::terminalplayer::one_play(file_name, args.volume, args.timer, args.repeat) }
}
