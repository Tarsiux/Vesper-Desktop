export interface Format {
  format_id: string | null;
  format: string | null;
  format_note: string | null;
  ext: string | null;
  protocol: string | null;
  acodec: string | null;
  vcodec: string | null;
  audio_ext: string | null;
  video_ext: string | null;
  url: string | null;
  width: number | null;
  height: number | null;
  fps: number | null;
  resolution: string | null;
  aspect_ratio: number | null;
  tbr: number | null;
  vbr: number | null;
  abr: number | null;
  asr: number | null;
  audio_channels: number | null;
  filesize: number | null;
  filesize_approx: number | null;
  has_drm: boolean | null;
  dynamic_range: string | null;
  container: string | null;
  language: string | null;
  quality: number | null;
  source_preference: number | null;
  preference: number | null;
  available_at: number | null;
  rows: number | null;
  columns: number | null;
  fragments: unknown[] | null;
  http_headers: unknown;
  downloader_options: unknown;
}

export interface VideoInfo {
  id: string;
  title: string;
  fulltitle: string | null;
  description: string | null;
  duration: number | null;
  duration_string: string | null;
  view_count: number | null;
  like_count: number | null;
  comment_count: number | null;
  channel: string | null;
  channel_id: string | null;
  channel_url: string | null;
  uploader: string | null;
  uploader_id: string | null;
  uploader_url: string | null;
  upload_date: string | null;
  timestamp: number | null;
  thumbnail: string | null;
  thumbnails: unknown[] | null;
  webpage_url: string | null;
  original_url: string | null;
  extractor: string | null;
  extractor_key: string | null;
  age_limit: number | null;
  categories: string[] | null;
  tags: string[] | null;
  live_status: string | null;
  availability: string | null;
  is_live: boolean | null;
  was_live: boolean | null;
  formats: Format[];
}

export interface DownloadOptions {
  fileName: string;
  videoFormatId: string | null;
  audioFormatId: string | null;
  videoExt: string;
  audioExt: string;
  merge: boolean;
  outputFormat: string;
}
