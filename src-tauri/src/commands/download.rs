use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{Emitter, Manager};

use crate::structs::{DownloadProgress, DownloadStatus, VideoInfo};

/// Nombre del evento emitido al frontend con el progreso en vivo.
const EVENT: &str = "download://progress";

/// Contador para que los ids de descarga sean únicos entre sí.
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Identifica una descarga activa para poder cancelarla: un flag de cancelación
/// y el pid del proceso hijo (yt-dlp/ffmpeg) que se está ejecutando ahora mismo.
#[derive(Clone)]
struct CancelHandle {
    flag: Arc<AtomicBool>,
    pid: Arc<Mutex<Option<u32>>>,
}

/// Descargas en curso, por id.
static ACTIVE: OnceLock<Mutex<HashMap<String, CancelHandle>>> = OnceLock::new();

fn active() -> &'static Mutex<HashMap<String, CancelHandle>> {
    ACTIVE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Mata el proceso y todo su árbol de hijos (yt-dlp puede lanzar ffmpeg interno).
fn kill_tree(pid: u32) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output();
    }
    #[cfg(not(windows))]
    {
        let _ = Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .output();
    }
}

fn new_id() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{millis}-{}", COUNTER.fetch_add(1, Ordering::Relaxed))
}

fn emit(
    app: &tauri::AppHandle,
    id: &str,
    status: DownloadStatus,
    progress: f64,
    message: Option<&str>,
    error: Option<&str>,
) {
    let _ = app.emit(
        EVENT,
        DownloadProgress {
            id: id.to_string(),
            status,
            progress,
            message: message.map(|s| s.to_string()),
            error: error.map(|s| s.to_string()),
        },
    );
}

/// Ejecuta yt-dlp leyendo su salida en vivo y emitiendo el porcentaje real de
/// descarga (líneas `[download]  xx.x%`), mapeado a la franja `range` del
/// progreso global (0-100).
///
/// Ojo: yt-dlp escribe el progreso en **stdout** y los errores en stderr. Hay
/// que drenar AMBOS pipes: si no se lee stdout, el buffer (64 KB) se llena,
/// yt-dlp se bloquea escribiendo y la descarga se queda colgada sin emitir
/// ningún evento.
fn run_ytdlp(
    cmd: &Path,
    args: &[String],
    cwd: &Path,
    app: &tauri::AppHandle,
    id: &str,
    range: (f64, f64),
    cancel: &CancelHandle,
) -> Result<(), String> {
    if cancel.flag.load(Ordering::SeqCst) {
        return Err("Descarga cancelada".into());
    }

    let mut child = Command::new(cmd)
        .current_dir(cwd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    *cancel.pid.lock().unwrap() = Some(child.id());

    // Si la cancelación llegó justo durante el spawn (antes de registrar el
    // pid), la matamos aquí mismo.
    if cancel.flag.load(Ordering::SeqCst) {
        kill_tree(child.id());
        let _ = child.wait();
        *cancel.pid.lock().unwrap() = None;
        return Err("Descarga cancelada".into());
    }

    // stderr (errores) se lee en un hilo aparte para no bloquear.
    let stderr = child.stderr.take().expect("stderr piped");
    let err_handle = std::thread::spawn(move || {
        let mut buf = String::new();
        for line in BufReader::new(stderr).lines() {
            if let Ok(l) = line {
                buf.push_str(&l);
                buf.push('\n');
            }
        }
        buf
    });

    // stdout (progreso) se lee aquí, en vivo.
    let stdout = child.stdout.take().expect("stdout piped");
    for line in BufReader::new(stdout).lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if let Some(pct) = parse_percent(&line) {
            let progress = range.0 + (pct / 100.0) * (range.1 - range.0);
            emit(app, id, DownloadStatus::Descargando, progress, None, None);
        }
    }

    let status = child.wait().map_err(|e| e.to_string())?;
    *cancel.pid.lock().unwrap() = None;
    let err_buf = err_handle.join().unwrap_or_default();

    if !status.success() {
        return Err(if err_buf.trim().is_empty() {
            "yt-dlp falló".into()
        } else {
            err_buf.trim().to_string()
        });
    }
    Ok(())
}

/// Ejecuta ffmpeg con `-progress pipe:1` y emite el progreso estimado según la
/// duración de la entrada (pasada desde el frontend). Sin duración disponible,
/// solo emite el inicio de la fase (el anillo queda en modo "procesando").
fn run_ffmpeg(
    cmd: &Path,
    args: &[String],
    cwd: &Path,
    app: &tauri::AppHandle,
    id: &str,
    status: DownloadStatus,
    range: (f64, f64),
    duration_secs: Option<f64>,
    cancel: &CancelHandle,
) -> Result<(), String> {
    if cancel.flag.load(Ordering::SeqCst) {
        return Err("Descarga cancelada".into());
    }

    let mut full_args = vec![
        "-nostats".into(),
        "-progress".into(),
        "pipe:1".into(),
    ];
    full_args.extend_from_slice(args);

    let mut child = Command::new(cmd)
        .current_dir(cwd)
        .args(&full_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    *cancel.pid.lock().unwrap() = Some(child.id());

    if cancel.flag.load(Ordering::SeqCst) {
        kill_tree(child.id());
        let _ = child.wait();
        *cancel.pid.lock().unwrap() = None;
        return Err("Descarga cancelada".into());
    }

    let stderr = child.stderr.take().expect("stderr piped");
    let err_handle = std::thread::spawn(move || {
        let mut buf = String::new();
        for line in BufReader::new(stderr).lines() {
            if let Ok(l) = line {
                buf.push_str(&l);
                buf.push('\n');
            }
        }
        buf
    });

    let stdout = child.stdout.take().expect("stdout piped");
    let duration = duration_secs.filter(|d| *d > 0.0);
    let mut out_time_us: f64 = 0.0;
    for line in BufReader::new(stdout).lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if let Some(v) = line.strip_prefix("out_time_us=") {
            if let Ok(v) = v.trim().parse::<f64>() {
                out_time_us = v;
            }
        }
        if let Some(d) = duration {
            let frac = (out_time_us / (d * 1_000_000.0)).clamp(0.0, 1.0);
            let progress = range.0 + frac * (range.1 - range.0);
            emit(app, id, status.clone(), progress, None, None);
        }
    }

    let result = child.wait().map_err(|e| e.to_string());
    *cancel.pid.lock().unwrap() = None;
    let err_buf = err_handle.join().unwrap_or_default();

    match result {
        Ok(s) if s.success() => Ok(()),
        Ok(_) => Err(if err_buf.trim().is_empty() {
            "ffmpeg falló".into()
        } else {
            err_buf.trim().to_string()
        }),
        Err(e) => Err(e),
    }
}

/// Extrae el porcentaje de una línea de progreso de yt-dlp
/// (`[download]  45.3% of ~2.5MiB at ...`).
fn parse_percent(line: &str) -> Option<f64> {
    let idx = line.find("[download]")?;
    let rest = line[idx + "[download]".len()..].trim_start();
    let bytes = rest.as_bytes();

    let mut i = 0;
    while i < bytes.len() && !bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i == bytes.len() {
        return None;
    }

    let start = i;
    while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
        i += 1;
    }

    let num: f64 = rest[start..i].parse().ok()?;
    if rest[i..].trim_start().starts_with('%') && (0.0..=100.0).contains(&num) {
        Some(num)
    } else {
        None
    }
}

fn find_output(dir: &Path, base: &str) -> Result<PathBuf, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| e.to_string())?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(base) {
            return Ok(entry.path());
        }
    }
    Err(format!("No se encontró el archivo descargado: {base}"))
}

/// ¿Hace falta pasar por ffmpeg? Solo si el archivo descargado no está ya en
/// la extensión de salida pedida (p. ej. un mp4 descargado con mp4 de salida):
/// en ese caso convertirlo sería un no-op y ffmpeg fallaría con
/// "same as Input" si la ruta de salida coincide con la de entrada.
fn needs_conversion(src: &Path, target: &Path) -> bool {
    let src_ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let tgt_ext = target
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    src_ext != tgt_ext
}

/// Mueve `src` a `dst` (borrando un `dst` previo si existiera, como haría
/// ffmpeg con `-y`). Si ya están en la misma ruta, no hace nada.
fn rename_if_needed(src: &Path, dst: &Path) -> Result<(), String> {
    if src == dst {
        return Ok(());
    }
    if dst.exists() {
        let _ = std::fs::remove_file(dst);
    }
    std::fs::rename(src, dst).map_err(|e| {
        format!(
            "No se pudo renombrar {} a {}: {}",
            src.display(),
            dst.display(),
            e
        )
    })
}

/// Convierte un nombre de archivo arbitrario en uno válido en Windows:
/// sustituye los caracteres prohibidos (`< > : " / \ | ? *`) y los de control
/// por `_`, elimina los espacios/puntos finales (prohibidos en Windows) y evita
/// los nombres reservados del sistema (CON, PRN, AUX, NUL, COM1-9, LPT1-9).
fn sanitize_windows_filename(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| {
            if matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || c.is_control()
            {
                '_'
            } else {
                c
            }
        })
        .collect();

    // Windows no permite espacios ni puntos al final del nombre.
    while out.ends_with(' ') || out.ends_with('.') {
        out.pop();
    }

    // Evita los nombres reservados de dispositivo.
    let stem = out.split('.').next().unwrap_or("").to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4
            && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && stem.as_bytes()[3].is_ascii_digit());
    if reserved {
        out.insert(0, '_');
    }

    let out = out.trim().to_string();
    if out.is_empty() {
        "descarga".to_string()
    } else {
        out
    }
}

fn video_codec(ext: &str) -> &'static str {
    match ext {
        "webm" => "libvpx-vp9",
        "avi" => "mpeg4",
        _ => "libx264",
    }
}

fn audio_codec(ext: &str) -> &'static str {
    match ext {
        "mp3" => "libmp3lame",
        "webm" | "opus" => "libopus",
        "wav" => "pcm_s16le",
        _ => "aac",
    }
}

fn binary_path(app: &tauri::AppHandle, name: &str) -> Result<PathBuf, String> {
    let file_name = format!("{name}.exe");

    if tauri::is_dev() {
        Ok(PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("binaries")
            .join(file_name))
    } else {
        let resource_dir = app.path().resource_dir().map_err(|e| e.to_string())?;
        Ok(resource_dir.join("binaries").join(file_name))
    }
}

#[tauri::command]
pub fn show_options_video(app: tauri::AppHandle, url: String) -> Result<VideoInfo, String> {
    let yt_dlp = binary_path(&app, "yt-dlp")?;

    let output = Command::new(yt_dlp)
        .arg("-J")
        .arg("--no-playlist")
        .arg(&url)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp falló: {}", stderr.trim()));
    }

    let value: serde_json::Value =
        serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;

    // Con --no-playlist, un enlace "video + playlist" devuelve solo el video.
    // Pero si el enlace es de una playlist completa, yt-dlp devuelve el JSON de
    // la playlist (no deserializable como VideoInfo): damos un mensaje claro.
    let kind = value
        .get("_type")
        .or_else(|| value.get("type"))
        .and_then(|t| t.as_str());
    if kind == Some("playlist") {
        return Err(
            "El enlace apunta a una playlist completa. Pega el enlace de un video concreto (o de un video dentro de la playlist)."
                .into(),
        );
    }
    if value.is_null() {
        return Err("No se encontró ningún video con esa URL".into());
    }

    let info: VideoInfo = serde_json::from_value(value).map_err(|e| e.to_string())?;

    Ok(info)
}

/// Lanza la descarga en un hilo separado y devuelve al instante un `id` que el
/// frontend usa para seguir el progreso vía eventos `download://progress`.
/// Así varias descargas corren en paralelo sin bloquear la UI.
#[tauri::command]
pub fn descargar(
    app: tauri::AppHandle,
    url: String,
    carpeta: String,
    file_name: String,
    video_format_id: Option<String>,
    audio_format_id: Option<String>,
    video_ext: String,
    audio_ext: String,
    merge: bool,
    output_format: String,
    duration: Option<f64>,
) -> Result<String, String> {
    let dir = PathBuf::from(&carpeta);

    if !dir.is_dir() {
        return Err("La carpeta de salida no existe".into());
    }

    let has_video = video_format_id
        .as_deref()
        .is_some_and(|s| !s.trim().is_empty());
    let has_audio = audio_format_id
        .as_deref()
        .is_some_and(|s| !s.trim().is_empty());

    if !has_video && !has_audio {
        return Err("Selecciona al menos un formato de video o de audio".into());
    }

    let id = new_id();
    let thread_id = id.clone();

    // Registramos la descarga como activa para poder cancelarla por id.
    let handle = CancelHandle {
        flag: Arc::new(AtomicBool::new(false)),
        pid: Arc::new(Mutex::new(None)),
    };
    active().lock().unwrap().insert(id.clone(), handle.clone());

    std::thread::spawn(move || {
        let result = run_download(
            &app,
            &thread_id,
            url,
            carpeta,
            file_name,
            video_format_id,
            audio_format_id,
            video_ext,
            audio_ext,
            merge,
            output_format,
            duration,
            has_video,
            has_audio,
            &handle,
        );
        active().lock().unwrap().remove(&thread_id);
        if let Err(e) = result {
            emit(&app, &thread_id, DownloadStatus::Error, 0.0, None, Some(&e));
        }
    });

    Ok(id)
}

/// Cancela una descarga en curso: mata el proceso (yt-dlp/ffmpeg) y el hilo
/// se encarga de borrar los archivos descargados hasta el momento.
#[tauri::command]
pub fn cancelar_descarga(id: String) {
    if let Some(handle) = active().lock().unwrap().remove(&id) {
        handle.flag.store(true, Ordering::SeqCst);
        if let Some(pid) = handle.pid.lock().unwrap().take() {
            kill_tree(pid);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_download(
    app: &tauri::AppHandle,
    id: &str,
    url: String,
    carpeta: String,
    file_name: String,
    video_format_id: Option<String>,
    audio_format_id: Option<String>,
    video_ext: String,
    audio_ext: String,
    merge: bool,
    output_format: String,
    duration: Option<f64>,
    has_video: bool,
    has_audio: bool,
    cancel: &CancelHandle,
) -> Result<(), String> {
    let result = run_download_inner(
        app,
        id,
        url,
        carpeta.clone(),
        file_name.clone(),
        video_format_id,
        audio_format_id,
        video_ext,
        audio_ext,
        merge,
        output_format,
        duration,
        has_video,
        has_audio,
        cancel,
    );

    // Si la descarga se canceló, elimina los archivos descargados hasta ahora
    // (parciales .part, video/audio ya bajados, etc.).
    if cancel.flag.load(Ordering::SeqCst) {
        cleanup_download_files(Path::new(&carpeta), id);
    }

    result
}

/// Borra todos los archivos temporales de una descarga cancelada: cualquier
/// archivo que empiece por `{id}_video` o `{id}_audio` (incluye los parciales
/// `.part` de yt-dlp).
fn cleanup_download_files(dir: &Path, base: &str) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with(&format!("{base}_video"))
            || name.starts_with(&format!("{base}_audio"))
        {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_download_inner(
    app: &tauri::AppHandle,
    id: &str,
    url: String,
    carpeta: String,
    file_name: String,
    video_format_id: Option<String>,
    audio_format_id: Option<String>,
    video_ext: String,
    audio_ext: String,
    merge: bool,
    output_format: String,
    duration: Option<f64>,
    has_video: bool,
    has_audio: bool,
    cancel: &CancelHandle,
) -> Result<(), String> {
    let yt_dlp = binary_path(app, "yt-dlp")?;
    let ffmpeg = binary_path(app, "ffmpeg")?;
    let dir = PathBuf::from(&carpeta);

    // El nombre final se sanea para que sea válido en Windows (el usuario puede
    // escribir `:` u otros caracteres prohibidos que romperían las rutas).
    let safe_name = sanitize_windows_filename(&file_name);
    // Los temporales usan el id como nombre interno neutro: yt-dlp nunca lo
    // modifica (a diferencia del nombre del usuario, que sanea por su cuenta),
    // así que siempre los encontramos; además dos descargas simultáneas con el
    // mismo nombre de archivo no colisionan.
    let video_out = format!("{id}_video.%(ext)s");
    let audio_out = format!("{id}_audio.%(ext)s");

    let mut video_file: Option<PathBuf> = None;
    let mut audio_file: Option<PathBuf> = None;

    // Descarga de video: ocupa del 0% al 40% (o al 70% si no hay audio).
    if has_video {
        let vid = video_format_id.unwrap();
        emit(
            app,
            id,
            DownloadStatus::Descargando,
            0.0,
            Some("Descargando video…"),
            None,
        );
        let range = if has_audio { (0.0, 40.0) } else { (0.0, 70.0) };
        run_ytdlp(
            &yt_dlp,
            &[
                "-f".into(),
                vid,
                "-o".into(),
                video_out,
                "--newline".into(),
                "--no-warnings".into(),
                "--no-playlist".into(),
                "--windows-filenames".into(),
                url.clone(),
            ],
            &dir,
            app,
            id,
            range,
            cancel,
        )?;
        video_file = Some(find_output(&dir, &format!("{id}_video"))?);
    }

    // Descarga de audio: del 40% al 70%.
    if has_audio {
        let aud = audio_format_id.unwrap();
        emit(
            app,
            id,
            DownloadStatus::Descargando,
            40.0,
            Some("Descargando audio…"),
            None,
        );
        run_ytdlp(
            &yt_dlp,
            &[
                "-f".into(),
                aud,
                "-o".into(),
                audio_out,
                "--newline".into(),
                "--no-warnings".into(),
                "--no-playlist".into(),
                "--windows-filenames".into(),
                url,
            ],
            &dir,
            app,
            id,
            (40.0, 70.0),
            cancel,
        )?;
        audio_file = Some(find_output(&dir, &format!("{id}_audio"))?);
    }

    match (&video_file, &audio_file) {
        (Some(vf), Some(af)) => {
            if merge {
                let final_path = dir.join(format!("{safe_name}.{output_format}"));
                println!("Juntando audio y video y convirtiendo a .{output_format}...");
                emit(
                    app,
                    id,
                    DownloadStatus::Uniendo,
                    70.0,
                    Some("Uniendo…"),
                    None,
                );
                run_ffmpeg(
                    &ffmpeg,
                    &[
                        "-y".into(),
                        "-i".into(),
                        vf.to_string_lossy().to_string(),
                        "-i".into(),
                        af.to_string_lossy().to_string(),
                        "-c:v".into(),
                        video_codec(&output_format).into(),
                        "-c:a".into(),
                        audio_codec(&output_format).into(),
                        "-shortest".into(),
                        final_path.to_string_lossy().to_string(),
                    ],
                    &dir,
                    app,
                    id,
                    DownloadStatus::Uniendo,
                    (70.0, 99.0),
                    duration,
                    cancel,
                )?;
                let _ = std::fs::remove_file(vf);
                let _ = std::fs::remove_file(af);
            } else {
                // Video por separado: solo se convierte si la extensión
                // descargada no es ya la pedida.
                let video_final = dir.join(format!("{safe_name}_video.{video_ext}"));
                if needs_conversion(vf, &video_final) {
                    println!("Convirtiendo video a .{video_ext}...");
                    emit(
                        app,
                        id,
                        DownloadStatus::Convirtiendo,
                        70.0,
                        Some("Convirtiendo…"),
                        None,
                    );
                    run_ffmpeg(
                        &ffmpeg,
                        &[
                            "-y".into(),
                            "-i".into(),
                            vf.to_string_lossy().to_string(),
                            "-c:v".into(),
                            video_codec(&video_ext).into(),
                            video_final.to_string_lossy().to_string(),
                        ],
                        &dir,
                        app,
                        id,
                        DownloadStatus::Convirtiendo,
                        (70.0, 84.5),
                        duration,
                        cancel,
                    )?;
                    let _ = std::fs::remove_file(vf);
                } else {
                    // Ya está en .{video_ext}: sin ffmpeg.
                    rename_if_needed(vf, &video_final)?;
                }

                // Audio por separado.
                let audio_final = dir.join(format!("{safe_name}_audio.{audio_ext}"));
                if needs_conversion(af, &audio_final) {
                    println!("Convirtiendo audio a .{audio_ext}...");
                    emit(
                        app,
                        id,
                        DownloadStatus::Convirtiendo,
                        84.5,
                        Some("Convirtiendo…"),
                        None,
                    );
                    run_ffmpeg(
                        &ffmpeg,
                        &[
                            "-y".into(),
                            "-i".into(),
                            af.to_string_lossy().to_string(),
                            "-c:a".into(),
                            audio_codec(&audio_ext).into(),
                            audio_final.to_string_lossy().to_string(),
                        ],
                        &dir,
                        app,
                        id,
                        DownloadStatus::Convirtiendo,
                        (84.5, 99.0),
                        duration,
                        cancel,
                    )?;
                    let _ = std::fs::remove_file(af);
                } else {
                    // Ya está en .{audio_ext}: sin ffmpeg.
                    rename_if_needed(af, &audio_final)?;
                }
            }
        }
        (Some(vf), None) => {
            // Solo video: si el archivo descargado ya está en la extensión
            // pedida, no hace falta ffmpeg (solo renombrar, o nada si ya tiene
            // el nombre final). Esto evita el fallo "same as Input" de ffmpeg
            // cuando la entrada y la salida son el mismo archivo.
            let (target, codec, label) = if merge {
                (
                    dir.join(format!("{safe_name}.{output_format}")),
                    video_codec(&output_format),
                    format!(".{output_format}"),
                )
            } else {
                (
                    dir.join(format!("{safe_name}_video.{video_ext}")),
                    video_codec(&video_ext),
                    format!(".{video_ext}"),
                )
            };
            if needs_conversion(vf, &target) {
                println!("Convirtiendo video a {label}...");
                emit(
                    app,
                    id,
                    DownloadStatus::Convirtiendo,
                    70.0,
                    Some("Convirtiendo…"),
                    None,
                );
                run_ffmpeg(
                    &ffmpeg,
                    &[
                        "-y".into(),
                        "-i".into(),
                        vf.to_string_lossy().to_string(),
                        "-c:v".into(),
                        codec.into(),
                        target.to_string_lossy().to_string(),
                    ],
                    &dir,
                    app,
                    id,
                    DownloadStatus::Convirtiendo,
                    (70.0, 99.0),
                    duration,
                    cancel,
                )?;
                let _ = std::fs::remove_file(vf);
            } else {
                rename_if_needed(vf, &target)?;
            }
        }
        (None, Some(af)) => {
            // Solo audio: igual que el caso de solo video.
            let (target, codec, label) = if merge {
                (
                    dir.join(format!("{safe_name}.{output_format}")),
                    audio_codec(&output_format),
                    format!(".{output_format}"),
                )
            } else {
                (
                    dir.join(format!("{safe_name}_audio.{audio_ext}")),
                    audio_codec(&audio_ext),
                    format!(".{audio_ext}"),
                )
            };
            if needs_conversion(af, &target) {
                println!("Convirtiendo audio a {label}...");
                emit(
                    app,
                    id,
                    DownloadStatus::Convirtiendo,
                    70.0,
                    Some("Convirtiendo…"),
                    None,
                );
                run_ffmpeg(
                    &ffmpeg,
                    &[
                        "-y".into(),
                        "-i".into(),
                        af.to_string_lossy().to_string(),
                        "-c:a".into(),
                        codec.into(),
                        target.to_string_lossy().to_string(),
                    ],
                    &dir,
                    app,
                    id,
                    DownloadStatus::Convirtiendo,
                    (70.0, 99.0),
                    duration,
                    cancel,
                )?;
                let _ = std::fs::remove_file(af);
            } else {
                rename_if_needed(af, &target)?;
            }
        }
        (None, None) => unreachable!("ya validado: al menos un formato"),
    }

    emit(
        app,
        id,
        DownloadStatus::Completado,
        100.0,
        Some("Completado"),
        None,
    );
    Ok(())
}
