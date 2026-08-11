pub mod folder;

pub fn register() -> Box<tauri::ipc::InvokeHandler<tauri::Wry>> {
    Box::new(tauri::generate_handler![
        folder::select_folder,

    ])
}