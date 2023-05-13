use std::path::PathBuf;

use clap::{Args, Parser};

use crate::format::Format;

#[derive(Parser)]
#[command(name = "windows-tts", version, about)]
pub(crate) struct Cli {
    /// The format of the input
    #[command(flatten)]
    pub input: Input,

    /// Optional output file to write the audio to instead of playing to audio device
    #[arg(short, long, value_parser, value_name = "FILE")]
    pub output: Option<PathBuf>,

    /// The format of the output file
    #[arg(short, long, value_enum, default_value_t = crate::format::Format::WAV)]
    pub format: Format,

    /// Whether to use SSML synthesis
    #[arg(long, default_value_t = false)]
    pub ssml: bool,
}

#[derive(Args)]
#[group(required = true)]
pub(crate) struct Input {
    /// Optional input file to read the audio from
    #[arg(short, long, value_parser, value_name = "FILE")]
    pub input: Option<PathBuf>,

    /// The message to create speech for
    #[arg(short, long, value_parser)]
    pub message: Option<String>,
}