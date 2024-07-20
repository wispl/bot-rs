use std::{borrow::Cow, error::Error, sync::Arc, time::Duration};

use async_trait::async_trait;

use songbird::input::{AudioStream, AudioStreamError, AuxMetadata, Compose, HttpRequest, Input};
use symphonia_core::io::MediaSource;

use reqwest::{header::HeaderMap, Client};

use yinfo::{Innertube, structs::VideoDetails};

struct StreamData {
    pub url: String,
    pub size: Option<String>,
}

pub struct RustYTDL<'a> {
    client: Client,
    innertube: Arc<Innertube>,
    query: Cow<'a, str>,
    metadata: Option<AuxMetadata>,
}

// TODO: handle playlist
impl<'a> RustYTDL<'a> {
    #[must_use]
    pub fn url(client: Client, innertube: Arc<Innertube>, url: impl Into<Cow<'a, str>>) -> Self {
        RustYTDL {
            client,
            innertube,
            query: url.into(),
            metadata: None,
        }
    }

    async fn query(&mut self) -> Result<StreamData, AudioStreamError> {
        let video = self
            .innertube
            .info(&self.query)
            .await
            .map_err(|e| AudioStreamError::Fail(Box::new(e)))?;

        let format = video
            .best_audio()
            .ok_or(AudioStreamError::Fail("No audio format found".into()))?;

        let url = self
            .innertube
            .decipher_format(format)
            .await
            .map_err(|e| AudioStreamError::Fail(Box::new(e)))?;

        let stream_data = StreamData {
            url,
            size: format.content_length.clone(),
        };

        self.metadata = Some(details_to_metadata(video.video_details));
        Ok(stream_data)
    }
}

impl From<RustYTDL<'static>> for Input {
    fn from(val: RustYTDL<'static>) -> Self {
        Input::Lazy(Box::new(val))
    }
}

#[async_trait]
impl<'a> Compose for RustYTDL<'a> {
    fn create(&mut self) -> Result<AudioStream<Box<dyn MediaSource>>, AudioStreamError> {
        Err(AudioStreamError::Unsupported)
    }

    async fn create_async(
        &mut self,
    ) -> Result<AudioStream<Box<dyn MediaSource>>, AudioStreamError> {
        let stream_data = self.query().await?;
        let content_length = stream_data.size.map(|s| s.parse::<u64>().unwrap());

        let mut req = HttpRequest {
            client: self.client.clone(),
            request: stream_data.url,
            headers: HeaderMap::default(),
            content_length,
        };

        req.create_async().await
    }

    fn should_create_async(&self) -> bool {
        true
    }

    async fn aux_metadata(&mut self) -> Result<AuxMetadata, AudioStreamError> {
        if let Some(meta) = self.metadata.as_ref() {
            return Ok(meta.clone());
        }

        let _ = self.query().await;

        self.metadata.clone().ok_or_else(|| {
            let msg: Box<dyn Error + Send + Sync + 'static> =
                "Failed to instansiate any metadata... Should be unreachable.".into();
            AudioStreamError::Fail(msg)
        })
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
