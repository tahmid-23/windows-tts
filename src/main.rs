use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;

use clap::Parser;
use futures::executor::block_on;
use futures::future;
use windows::core::HSTRING;
use windows::Foundation::TypedEventHandler;
use windows::Media::Core::MediaSource;
use windows::Media::Playback::MediaPlayer;
use windows::Media::SpeechSynthesis::{SpeechSynthesisStream, SpeechSynthesizer};
use windows::Storage::{CreationCollisionOption, FileIO, StorageFile, StorageFolder};
use windows::Storage::Streams::{DataReader, IBuffer};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Parser)]
#[clap(version, about)]
struct Args {
    /// Optional output file to write the audio to in .wav format instead of playing to audio device
    #[clap(short, long, value_parser, value_name = "FILE")]
    output: Option<PathBuf>,

    /// The message to create speech for
    #[clap(short, long, value_parser)]
    message: String,
}

#[derive(Debug)]
struct NoPathParent;

impl Display for NoPathParent {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid path parent")
    }
}

impl Error for NoPathParent {}

#[derive(Debug)]
struct NoFileName;

impl Display for NoFileName {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid file name")
    }
}

impl Error for NoFileName {}

async fn get_file(path: &Path) -> Result<StorageFile> {
    let folder_path = HSTRING::from(path.parent().ok_or_else(|| NoPathParent)?.as_os_str());
    let folder = StorageFolder::GetFolderFromPathAsync(&folder_path)?.await?;

    let file_name = &HSTRING::from(path.file_name().ok_or_else(|| NoFileName)?);
    Ok(folder.CreateFileAsync(&file_name, CreationCollisionOption::ReplaceExisting)?.await?)
}

async fn create_synth_text_buffer(message: &str) -> Result<IBuffer> {
    let synth: SpeechSynthesizer = SpeechSynthesizer::new()?;
    let synth_stream: SpeechSynthesisStream = synth.SynthesizeTextToStreamAsync(&HSTRING::from(message))?.await?;

    let reader = DataReader::CreateDataReader(&synth_stream)?;
    let stream_size: u32 = synth_stream.Size()?.try_into()?;
    reader.LoadAsync(stream_size)?;
    Ok(reader.ReadBuffer(stream_size)?)
}

async fn speak_to_file(message: &str, path: &Path) -> Result<()> {
    let file_future = get_file(path);
    let synth_text_buffer_future = create_synth_text_buffer(message);

    let results = future::join(file_future, synth_text_buffer_future).await;
    let file = results.0?;
    let buffer = results.1?;
    FileIO::WriteBufferAsync(&file, &buffer)?.await?;

    Ok(())
}

async fn speak_to_media_player(message: &str) -> Result<()> {
    let synth = SpeechSynthesizer::new()?;
    let mp = MediaPlayer::new()?;

    let synth_stream = synth.SynthesizeTextToStreamAsync(&HSTRING::from(message))?.await?;
    let source = MediaSource::CreateFromStream(&synth_stream, &synth_stream.ContentType()?)?;

    let (tx, rx) = channel();
    let end_token = mp.MediaEnded(&TypedEventHandler::new(move |_sender, _args| {
        tx.send(()).unwrap();
        Ok(())
    }))?;
    mp.SetSource(&source)?;
    mp.Play()?;

    rx.recv()?;
    mp.RemoveMediaEnded(end_token)?;

    Ok(())
}

fn main() -> Result<()> {
    let args = Args::parse();
    match args.output {
        Some(path) => {
            block_on(speak_to_file(&args.message, &path))
        }
        None => {
            block_on(speak_to_media_player(&args.message))
        }
    }
}
