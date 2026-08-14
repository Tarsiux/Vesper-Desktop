pub mod folder;
pub mod download;
pub mod editor;

pub fn register() -> Box<tauri::ipc::InvokeHandler<tauri::Wry>> {
    Box::new(tauri::generate_handler![
        folder::select_folder,
        download::show_options_video,
        download::download,
        download::cancel_download,
        download::update_ytdlp,
        editor::select_media_file,
        editor::generate_thumbnails,
        editor::trim_media,
        editor::cancel_trim,
    ])
}