use std::path::PathBuf;

use clap::Parser;

use crate::format::Format;

#[derive(Parser)]
#[command(name = "WindowsTTS", version, about)]
pub(crate) struct Cli {
    /// Optional output file to write the audio to in .wav format instead of playing to audio device
    #[arg(short, long, value_parser, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// The message to create speech for
    #[arg(short, long, value_parser)]
    pub message: String,

    #[arg(short, long, value_enum, default_value_t = crate::format::Format::WAV)]
    pub format: Format,
}