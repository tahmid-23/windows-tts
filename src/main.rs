use std::env::current_dir;
use std::error::Error;
use std::path::Path;
use std::sync::mpsc::channel;

use clap::Parser;
use futures::executor::block_on;
use futures::future;
use windows::core::HSTRING;
use windows::Foundation::TypedEventHandler;
use windows::Media::Core::MediaSource;
use windows::Media::MediaProperties::{AudioEncodingQuality, MediaEncodingProfile};
use windows::Media::Playback::MediaPlayer;
use windows::Media::SpeechSynthesis::SpeechSynthesizer;
use windows::Media::Transcoding::MediaTranscoder;
use windows::Storage::{CreationCollisionOption, FileIO, StorageFile, StorageFolder};
use windows::Storage::Streams::{DataReader, IBuffer, InMemoryRandomAccessStream, IRandomAccessStream, IRandomAccessStreamWithContentType};

use crate::cli::Cli;
use crate::error::{NoFileName, NoPathParent};
use crate::format::Format;

mod cli;
mod error;
mod format;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

async fn get_file(path: &Path) -> Result<StorageFile> {
    let folder_path = HSTRING::from(path.parent().ok_or(NoPathParent)?.as_os_str());
    let folder = StorageFolder::GetFolderFromPathAsync(&folder_path)?.await?;

    let file_name = HSTRING::from(path.file_name().ok_or(NoFileName)?);
    Ok(folder.CreateFileAsync(&file_name, CreationCollisionOption::ReplaceExisting)?.await?)
}

async fn create_synth_text_stream(message: &str) -> Result<IRandomAccessStreamWithContentType> {
    let synth = SpeechSynthesizer::new()?;
    Ok(IRandomAccessStreamWithContentType::try_from(synth.SynthesizeTextToStreamAsync(&HSTRING::from(message))?.await?)?)
}

async fn buffer_from_stream(stream: &IRandomAccessStream) -> Result<IBuffer> {
    let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
    let stream_size: u32 = stream.Size()?.try_into()?;
    reader.LoadAsync(stream_size)?.await?;

    Ok(reader.ReadBuffer(stream_size)?)
}

fn choose_profile(format: &Format) -> Result<Option<MediaEncodingProfile>> {
    match format {
        Format::WAV => {
            Ok(None)
        }
        Format::MP3 => {
            Ok(Some(MediaEncodingProfile::CreateMp3(AudioEncodingQuality::default())?))
        }
    }
}

async fn create_synth_text_buffer(message: &str, format: &Format) -> Result<IBuffer> {
    let input_stream = create_synth_text_stream(message).await?;

    let profile = choose_profile(format)?;
    match profile {
        Some(profile) => {
            let transcoder: MediaTranscoder = MediaTranscoder::new()?;
            let output_stream = IRandomAccessStream::try_from(InMemoryRandomAccessStream::new()?)?;
            let transcode_result: windows::Media::Transcoding::PrepareTranscodeResult = transcoder.PrepareStreamTranscodeAsync(&input_stream, &output_stream, &profile)?.await?;

            if transcode_result.CanTranscode()? {
                transcode_result.TranscodeAsync()?.await?
            }
            buffer_from_stream(&output_stream).await // TODO
        }
        None => {
            buffer_from_stream(&IRandomAccessStream::try_from(input_stream)?).await
        }
    }
}

async fn speak_to_file(message: &str, path: &Path, output_format: &Format) -> Result<()> {
    let file_future = get_file(path);
    let synth_text_buffer_future = create_synth_text_buffer(message, output_format);


    let results = future::join(file_future, synth_text_buffer_future).await;
    let file = results.0?;
    let buffer = results.1?;
    FileIO::WriteBufferAsync(&file, &buffer)?.await?;

    Ok(())
}

async fn speak_to_media_player(message: &str) -> Result<()> {
    let media_player = MediaPlayer::new()?;

    let synth_stream = create_synth_text_stream(message).await?;
    let source = MediaSource::CreateFromStream(&synth_stream, &synth_stream.ContentType()?)?;

    let (tx, rx) = channel();
    let end_token = media_player.MediaEnded(&TypedEventHandler::new(move |_sender, _args| {
        tx.send(()).unwrap();
        Ok(())
    }))?;
    media_player.SetSource(&source)?;
    media_player.Play()?;

    rx.recv()?;
    media_player.RemoveMediaEnded(end_token)?;

    Ok(())
}

fn main() -> Result<()> {
    let args = Cli::parse();
    match args.output {
        Some(path) => {
            let adjusted_path = match current_dir() {
                Ok(current) => current.join(path),
                Err(_) => path
            };
            block_on(speak_to_file(&args.message, &adjusted_path, &args.format))
        }
        None => {
            block_on(speak_to_media_player(&args.message))
        }
    }
}
