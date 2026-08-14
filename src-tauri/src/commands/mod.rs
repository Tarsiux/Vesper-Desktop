pub mod folder;
pub mod download;

pub fn register() -> Box<tauri::ipc::InvokeHandler<tauri::Wry>> {
    Box::new(tauri::generate_handler![
        folder::select_folder,
        download::show_options_video,
        download::descargar,
        download::cancelar_descarga,
    ])
}