use std::env::current_dir;
use std::error::Error;
use std::path::Path;
use std::sync::mpsc::channel;

use clap::Parser;
use futures::executor::block_on;
use futures::future;
use windows::core::{ComInterface, HSTRING};
use windows::Foundation::TypedEventHandler;
use windows::Media::Core::MediaSource;
use windows::Media::MediaProperties::{AudioEncodingQuality, MediaEncodingProfile};
use windows::Media::Playback::MediaPlayer;
use windows::Media::SpeechSynthesis::SpeechSynthesizer;
use windows::Media::Transcoding::MediaTranscoder;
use windows::Storage::{CreationCollisionOption, FileIO, IStorageFolder, StorageFile, StorageFolder};
use windows::Storage::Streams::{DataReader, IBuffer, InMemoryRandomAccessStream, IRandomAccessStream, IRandomAccessStreamWithContentType};

use crate::cli::Cli;
use crate::error::{NoFileName, NoPathParent, TranscodeFailed};
use crate::format::Format;

mod cli;
mod error;
mod format;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

async fn get_folder_and_file_name(path: &Path) -> Result<(IStorageFolder, HSTRING)> {
    let folder_path = HSTRING::from(path.parent().ok_or(NoPathParent)?.as_os_str());

    let folder = StorageFolder::GetFolderFromPathAsync(&folder_path)?.await?.cast::<IStorageFolder>()?;
    let file_name = HSTRING::from(path.file_name().ok_or(NoFileName)?);

    return Ok((folder, file_name));
}

async fn create_file(path: &Path) -> Result<StorageFile> {
    let (folder, file_name) = get_folder_and_file_name(path).await?;
    Ok(folder.CreateFileAsync(&file_name, CreationCollisionOption::ReplaceExisting)?.await?)
}

async fn get_file(path: &Path) -> Result<StorageFile> {
    let (folder, file_name) = get_folder_and_file_name(path).await?;
    Ok(folder.GetFileAsync(&file_name)?.await?)
}

async fn create_synth_text_stream(message: &HSTRING) -> Result<IRandomAccessStreamWithContentType> {
    let synth = SpeechSynthesizer::new()?;
    Ok(synth.SynthesizeTextToStreamAsync(message)?.await?.cast()?)
}

async fn buffer_from_stream(stream: &IRandomAccessStream) -> Result<IBuffer> {
    let reader = DataReader::CreateDataReader(&stream.GetInputStreamAt(0)?)?;
    let stream_size: u32 = stream.Size()?.try_into()?;
    reader.LoadAsync(stream_size)?.await?;

    Ok(reader.ReadBuffer(stream_size)?)
}

fn choose_profile(format: &Format) -> Result<Option<MediaEncodingProfile>> {
    Ok(match format {
        Format::ALAC => {
            Some(MediaEncodingProfile::CreateAlac(AudioEncodingQuality::default())?)
        }
        Format::FLAC => {
            Some(MediaEncodingProfile::CreateFlac(AudioEncodingQuality::default())?)
        }
        Format::M4A => {
            Some(MediaEncodingProfile::CreateM4a(AudioEncodingQuality::default())?)
        }
        Format::MP3 => {
            Some(MediaEncodingProfile::CreateMp3(AudioEncodingQuality::default())?)
        }
        Format::WAV => {
            None
        }
        Format::WMA => {
            Some(MediaEncodingProfile::CreateWma(AudioEncodingQuality::default())?)
        }
    })
}

async fn create_synth_text_buffer(message: &HSTRING, format: &Format) -> Result<IBuffer> {
    let input_stream = create_synth_text_stream(message).await?;

    let profile = choose_profile(format)?;
    match profile {
        Some(profile) => {
            let transcoder = MediaTranscoder::new()?;
            let output_stream = InMemoryRandomAccessStream::new()?.cast::<IRandomAccessStream>()?;
            let transcode_result = transcoder.PrepareStreamTranscodeAsync(&input_stream, &output_stream, &profile)?.await?;

            if transcode_result.CanTranscode()? {
                transcode_result.TranscodeAsync()?.await?;
                buffer_from_stream(&output_stream).await
            } else {
                Err(Box::new(TranscodeFailed))
            }
        }
        None => {
            buffer_from_stream(&input_stream.cast()?).await
        }
    }
}

async fn speak_to_file(message: &HSTRING, path: &Path, output_format: &Format) -> Result<()> {
    let file_future = create_file(path);
    let synth_text_buffer_future = create_synth_text_buffer(message, output_format);

    let results = future::join(file_future, synth_text_buffer_future).await;
    FileIO::WriteBufferAsync(&results.0?, &results.1?)?.await?;

    Ok(())
}

async fn speak_to_media_player(message: &HSTRING) -> Result<()> {
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
    let input_text = match args.input.message {
        Some(message) => {
            HSTRING::from(message)
        }
        None => {
            let adjusted_path = match current_dir() {
                Ok(current) => current.join(args.input.input.unwrap()),
                Err(_) => args.input.input.unwrap()
            };

            let file = block_on(get_file(&adjusted_path))?;
            block_on(FileIO::ReadTextAsync(&file)?)?
        }
    };

    match args.output {
        Some(path) => {
            let adjusted_path = match current_dir() {
                Ok(current) => current.join(path),
                Err(_) => path
            };
            block_on(speak_to_file(&input_text, &adjusted_path, &args.format))
        }
        None => {
            block_on(speak_to_media_player(&input_text))
        }
    }
}
