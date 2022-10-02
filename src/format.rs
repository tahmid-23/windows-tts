use clap::ValueEnum;

#[derive(Clone, ValueEnum)]
pub(crate) enum Format {
    WAV,
    MP3
}