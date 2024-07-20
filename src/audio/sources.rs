use std::time::Duration;

use async_trait::async_trait;

use songbird::input::{AudioStream, AudioStreamError, AuxMetadata, Compose, HttpRequest, Input};
use symphonia_core::io::MediaSource;

use reqwest::{header::HeaderMap, Client};

use yinfo::{structs::VideoDetails, Innertube};

/// A similar struct to [`songbird::input::YoutubeDl`], though only for YouTube links.
///
/// However there are some differences. Calling [`YouTube::new()`] immediately creates a request to
/// fetch metadata, and not on the first call to [`Compose::aux_metadata()`]. The reason for the
/// immediate request is to find if the video is playable before enqueueing the track instead of
/// when we reach it in the queue.
///
/// This also means metadata is guaranteed since it is extracted during the initial request.
pub struct YouTube {
    client: Client,
    metadata: AuxMetadata,
    file_size: Option<String>,
    stream_url: String,
}

impl YouTube {
    /// Creates a new YouTube source for the given video id or url.
    ///
    /// The request to the extracted stream url uses the passed in client.
    pub async fn new(
        innertube: &Innertube,
        client: Client,
        url: &str,
    ) -> Result<Self, AudioStreamError> {
        let video = innertube
            .info(url)
            .await
            .map_err(|e| AudioStreamError::Fail(Box::new(e)))?;

        if video.playability_status.status != "OK" {
            return Err(AudioStreamError::Fail("Video is unavailable.".into()));
        }

        let format = video
            .best_audio()
            .ok_or(AudioStreamError::Fail("No formats found".into()))?;
        let stream_url = innertube
            .decipher_format(format)
            .await
            .map_err(|e| AudioStreamError::Fail(Box::new(e)))?;

        let file_size = format.content_length.clone();
        let metadata = details_to_metadata(video.video_details);

        Ok(YouTube {
            client,
            metadata,
            file_size,
            stream_url,
        })
    }
}

impl From<YouTube> for Input {
    fn from(val: YouTube) -> Self {
        Input::Lazy(Box::new(val))
    }
}

#[async_trait]
impl Compose for YouTube {
    fn create(&mut self) -> Result<AudioStream<Box<dyn MediaSource>>, AudioStreamError> {
        Err(AudioStreamError::Unsupported)
    }

    async fn create_async(
        &mut self,
    ) -> Result<AudioStream<Box<dyn MediaSource>>, AudioStreamError> {
        let content_length = self.file_size.as_ref().map(|s| s.parse::<u64>().unwrap());

        let mut req = HttpRequest {
            client: self.client.clone(),
            request: self.stream_url.clone(),
            headers: HeaderMap::default(),
            content_length,
        };

        req.create_async().await
    }

    fn should_create_async(&self) -> bool {
        true
    }

    async fn aux_metadata(&mut self) -> Result<AuxMetadata, AudioStreamError> {
        Ok(self.metadata.clone())
    }
}

fn details_to_metadata(details: VideoDetails) -> AuxMetadata {
    let length = details.length_seconds.parse::<u64>().unwrap();
    let thumbnail = details.thumbnails.thumbnails.first().unwrap().url.clone();
    let url = "https://www.youtube.com/watch?v=".to_owned() + &details.video_id;

    AuxMetadata {
        track: None,
        artist: None,
        album: None,
        // date: Some(details.upload_date),
        date: None,
        channels: Some(2),
        channel: Some(details.author),
        duration: Some(Duration::from_secs(length)),
        source_url: Some(url),
        title: Some(details.title),
        thumbnail: Some(thumbnail),

        ..AuxMetadata::default()
    }
}
