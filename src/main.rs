mod core;
mod terminal;
mod interactive;
use std::io::IsTerminal;
use std::process::ExitCode;
use clap::Parser;

#[derive(Debug,Parser)]
#[command(author,version,about,arg_required_else_help = true)]
#[clap(
long_about = "Neiro / Simple Audio File Player"
)]
struct Cli {
    #[clap(
        help = "Audio file path",
        value_name = "FILE",
        required = true,
    )]
    file_name:String,
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
        help = "Run in TUI mode with UI controls (default)",
        long = "tui",
        alias = "interactive",
        short_alias = 'i',
        conflicts_with = "mode_cli",
    )]
    mode_tui:bool,
    #[clap(
        help = "Run in CLI mode without UI",
        long = "cli",
        short = 'c',
    )]
    mode_cli:bool,
}

fn main() -> ExitCode {
    let args = Cli::parse();
    match
        // 標準はTUIモード。パイプやリダイレクトなど端末でない場合はCLIモードで再生する
        if args.mode_tui || (!args.mode_cli && std::io::stdout().is_terminal()) {
            interactive::run::run_interactive_player(&args.file_name, args.volume, args.timer)
        }
        else { terminal::terminalplayer::one_play(&args.file_name, args.volume, args.timer) }
    {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}",e);
            ExitCode::FAILURE
        }
    }
}