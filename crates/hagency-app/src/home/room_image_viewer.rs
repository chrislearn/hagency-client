use std::sync::{Arc, Mutex};

use makepad_widgets::*;
use matrix_sdk_ui::timeline::EventTimelineItem;
use matrix_sdk::{
    media::{MediaFormat, MediaRequestParameters},
    ruma::events::room::{message::MessageType, MediaSource},
};
use matrix_sdk::reqwest::StatusCode;

use crate::{
    media_cache::{error_to_media_cache_entry, MediaCacheEntry},
    shared::image_viewer::ImageViewerError,
    sliding_sync::{submit_async_request, MatrixRequest},
};

/// Fetches the full-size image for the image viewer.
///
/// The image viewer is already showing a thumbnail, so this returns nothing.
/// [`show_fetched_image_in_viewer`] will be called when the full image arrives.
pub fn fetch_full_image_for_viewer(media_source: MediaSource) {
    submit_async_request(MatrixRequest::FetchMedia {
        media_request: MediaRequestParameters {
            source: media_source,
            format: MediaFormat::File,
        },
        on_fetched: show_fetched_image_in_viewer,
        // this isn't used
        destination: Arc::new(Mutex::new(MediaCacheEntry::Requested)),
        update_sender: None,
    });
}

/// Gets the image's file name and size in bytes from an event timeline item.
pub fn get_image_name_and_filesize(event_tl_item: &EventTimelineItem) -> (String, u64) {
    if let Some(message) = event_tl_item.content().as_message() {
        if let MessageType::Image(image_content) = message.msgtype() {
            let name = image_content.filename().to_string();
            let size = image_content
                .info
                .as_ref()
                .and_then(|info| info.size)
                .map_or(0, u64::from);
            return (name, size);
        }
    }
    ("Unknown Image".to_string(), 0)
}

/// The result of the image viewer's request to fetch a full-size image.
#[derive(Clone, Debug)]
pub enum ImageViewerFetchAction {
    Loaded(matrix_sdk::ruma::OwnedMxcUri, Arc<[u8]>),
    Failed(matrix_sdk::ruma::OwnedMxcUri, ImageViewerError),
}

fn show_fetched_image_in_viewer(
    _destination: &Mutex<MediaCacheEntry>,
    request: MediaRequestParameters,
    data: matrix_sdk::Result<Vec<u8>>,
    _update_sender: Option<crate::timeline_updates::Sender>,
) {
    Cx::post_action(fetched_action(&request, data));
}

fn fetched_action(
    request: &MediaRequestParameters,
    data: matrix_sdk::Result<Vec<u8>>,
) -> ImageViewerFetchAction {
    let source = crate::shared::attachment_download::media_source_mxc(&request.source).to_owned();
    match data {
        Ok(data) => {
            let bytes: Arc<[u8]> = data.into();
            if crate::memory_budget::admit_encoded(&bytes) {
                ImageViewerFetchAction::Loaded(source, bytes)
            } else {
                ImageViewerFetchAction::Failed(source, ImageViewerError::TooLarge)
            }
        }
        Err(e) => ImageViewerFetchAction::Failed(
            source,
            match error_to_media_cache_entry(e, request) {
                MediaCacheEntry::Failed(StatusCode::PAYLOAD_TOO_LARGE) => {
                    ImageViewerError::TooLarge
                }
                MediaCacheEntry::Failed(StatusCode::NOT_FOUND) => ImageViewerError::NotFound,
                MediaCacheEntry::Failed(StatusCode::INTERNAL_SERVER_ERROR) => {
                    ImageViewerError::ConnectionFailed
                }
                MediaCacheEntry::Failed(StatusCode::PARTIAL_CONTENT) => ImageViewerError::BadData,
                MediaCacheEntry::Failed(StatusCode::UNAUTHORIZED) => ImageViewerError::Unauthorized,
                _ => ImageViewerError::Unknown,
            },
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oversized_viewer_result_is_rejected_before_action_queueing() {
        let request = MediaRequestParameters {
            source: MediaSource::Plain("mxc://example.org/large".into()),
            format: MediaFormat::File,
        };
        assert!(matches!(
            fetched_action(
                &request,
                Ok(vec![0; crate::memory_budget::IMAGE_FILE_BYTES + 1])
            ),
            ImageViewerFetchAction::Failed(_, ImageViewerError::TooLarge)
        ));
    }
}
