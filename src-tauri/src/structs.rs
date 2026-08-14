use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Format {
    pub format_id: Option<String>,
    pub format: Option<String>,
    pub format_note: Option<String>,
    pub ext: Option<String>,
    pub protocol: Option<String>,
    pub acodec: Option<String>,
    pub vcodec: Option<String>,
    pub audio_ext: Option<String>,
    pub video_ext: Option<String>,
    pub url: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub fps: Option<f64>,
    pub resolution: Option<String>,
    pub aspect_ratio: Option<f64>,
    pub tbr: Option<f64>,
    pub vbr: Option<f64>,
    pub abr: Option<f64>,
    pub asr: Option<f64>,
    pub audio_channels: Option<f64>,
    pub filesize: Option<f64>,
    pub filesize_approx: Option<f64>,
    pub has_drm: Option<bool>,
    pub dynamic_range: Option<String>,
    pub container: Option<String>,
    pub language: Option<String>,
    pub quality: Option<f64>,
    pub source_preference: Option<f64>,
    pub preference: Option<f64>,
    pub available_at: Option<f64>,
    pub rows: Option<f64>,
    pub columns: Option<f64>,
    pub fragments: Option<Vec<serde_json::Value>>,
    pub http_headers: Option<serde_json::Value>,
    pub downloader_options: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DownloadStatus {
    Descargando,
    Convirtiendo,
    Uniendo,
    Completado,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct DownloadProgress {
    pub id: String,
    pub status: DownloadStatus,
    pub progress: f64,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateProgress {
    pub progress: f64,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EditStatus {
    Procesando,
    Completado,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct EditProgress {
    pub id: String,
    pub status: EditStatus,
    pub progress: f64,
    pub message: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VideoInfo {
    pub id: String,
    pub title: String,
    pub fulltitle: Option<String>,
    pub description: Option<String>,
    pub duration: Option<f64>,
    pub duration_string: Option<String>,
    pub view_count: Option<f64>,
    pub like_count: Option<f64>,
    pub comment_count: Option<f64>,
    pub channel: Option<String>,
    pub channel_id: Option<String>,
    pub channel_url: Option<String>,
    pub uploader: Option<String>,
    pub uploader_id: Option<String>,
    pub uploader_url: Option<String>,
    pub upload_date: Option<String>,
    pub timestamp: Option<f64>,
    pub thumbnail: Option<String>,
    pub thumbnails: Option<Vec<serde_json::Value>>,
    pub webpage_url: Option<String>,
    pub original_url: Option<String>,
    pub extractor: Option<String>,
    pub extractor_key: Option<String>,
    pub age_limit: Option<f64>,
    pub categories: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub live_status: Option<String>,
    pub availability: Option<String>,
    pub is_live: Option<bool>,
    pub was_live: Option<bool>,
    pub formats: Vec<Format>,
}
