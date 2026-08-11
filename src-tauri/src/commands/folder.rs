use rfd::FileDialog;

#[tauri::command]
pub fn select_folder() -> Result<Option<String>, String> {
    let folder = FileDialog::new()
        .set_title("Seleccionar carpeta")
        .pick_folder();

    Ok(folder.map(|path| path.display().to_string().replace('\\', "/")))
}