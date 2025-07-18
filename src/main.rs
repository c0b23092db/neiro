mod core;
mod terminal;
mod interactive;
use std::process::ExitCode;
use clap::Parser;

#[derive(Debug,Parser)]
#[command(author,version,about,arg_required_else_help = true)]
#[clap(
long_about = "NeiroKiku / Simple Audio File Player"
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
        help = "Volume of Audio file [0-100]",
        long = "volume",
        short = 'v',
        default_value_t = 50,
    )]
    volume:u8,
    #[clap(
        help = "Run in Interactive mode with UI controls",
        long = "interactive",
        short = 'i',
    )]
    mode_interactive:bool,
}

fn main() -> ExitCode {
    let args = Cli::parse();
    match
        if args.mode_interactive { interactive::run::run_interactive_player(&args.file_name, args.volume) }
        else { terminal::terminalplayer::one_play(&args.file_name, args.volume, args.timer) }
    {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}",e);
            ExitCode::FAILURE
        }
    }
}