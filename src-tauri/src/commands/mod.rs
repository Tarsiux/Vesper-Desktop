pub mod folder;
pub mod download;
pub mod editor;

pub fn register() -> Box<tauri::ipc::InvokeHandler<tauri::Wry>> {
    Box::new(tauri::generate_handler![
        folder::select_folder,
        download::show_options_video,
        download::descargar,
        download::cancelar_descarga,
        download::actualizar_ytdlp,
        editor::select_media_file,
        editor::generar_thumbnails,
        editor::recortar_media,
        editor::cancelar_recorte,
    ])
}