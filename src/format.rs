use clap::ValueEnum;

#[derive(Clone, ValueEnum)]
pub(crate) enum Format {
    ALAC,
    FLAC,
    M4A,
    MP3,
    WAV,
    WMA,
}