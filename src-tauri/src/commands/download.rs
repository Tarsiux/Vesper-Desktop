use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::{Emitter, Manager};

use crate::structs::{DownloadProgress, DownloadStatus, UpdateProgress, VideoInfo};

// Allows spawning processes without a console window on Windows (CREATE_NO_WINDOW).
#[cfg(windows)]
use std::os::windows::process::CommandExt;

/// Name of the event emitted to the frontend with live progress.
const EVENT: &str = "download://progress";

/// Counter so download ids stay unique among themselves.
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Identifies an active download so it can be cancelled: a cancellation flag
/// and the pid of the currently running child process (yt-dlp/ffmpeg).
#[derive(Clone)]
struct CancelHandle {
    flag: Arc<AtomicBool>,
    pid: Arc<Mutex<Option<u32>>>,
}

/// Active downloads, keyed by id.
static ACTIVE: OnceLock<Mutex<HashMap<String, CancelHandle>>> = OnceLock::new();

fn active() -> &'static Mutex<HashMap<String, CancelHandle>> {
    ACTIVE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Creates a `Command` that does not open a console window when spawning
/// console processes (yt-dlp/ffmpeg/taskkill) from the GUI app. In release the
/// app is a GUI (no console) and Windows, without this flag, opens a new cmd
/// window for every child process; `CREATE_NO_WINDOW` (0x08000000) suppresses it.
pub(crate) fn hidden_cmd(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

/// Kills the process and its whole child tree (yt-dlp may spawn an internal ffmpeg).
pub(crate) fn kill_tree(pid: u32) {
    #[cfg(windows)]
    {
        let _ = hidden_cmd("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output();
    }
    #[cfg(not(windows))]
    {
        let _ = hidden_cmd("kill")
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

/// Runs yt-dlp reading its output live and emitting the real download
/// percentage (`[download]  xx.x%` lines), mapped to the `range` slice of the
/// overall progress (0-100).
///
/// Note: yt-dlp writes progress to **stdout** and errors to stderr. Both pipes
/// must be drained: if stdout is not read, the buffer (64 KB) fills up,
/// yt-dlp blocks while writing and the download hangs without emitting any
/// event.
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

    let mut child = hidden_cmd(cmd)
        .current_dir(cwd)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    *cancel.pid.lock().unwrap() = Some(child.id());

    // If the cancellation arrived right during spawn (before the pid was
    // registered), kill it here.
    if cancel.flag.load(Ordering::SeqCst) {
        kill_tree(child.id());
        let _ = child.wait();
        *cancel.pid.lock().unwrap() = None;
        return Err("Descarga cancelada".into());
    }

    // stderr (errors) is drained on a separate thread so nothing blocks.
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

    // stdout (progress) is read here, live.
    let stdout = child.stdout.take().expect("stdout piped");
    for line in BufReader::new(stdout).lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if let Some(pct) = parse_percent(&line) {
            let progress = range.0 + (pct / 100.0) * (range.1 - range.0);
            emit(app, id, DownloadStatus::Downloading, progress, None, None);
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

/// Runs ffmpeg with `-progress pipe:1` and emits the estimated progress based
/// on the input duration (passed from the frontend). Without a duration, only
/// the phase start is emitted (the ring stays in "processing" mode).
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

    let mut child = hidden_cmd(cmd)
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

/// Extracts the percentage from a yt-dlp progress line
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

/// Is ffmpeg needed? Only when the downloaded file is not already in the
/// requested output extension (e.g. an mp4 downloaded with mp4 as output):
/// converting would be a no-op and ffmpeg would fail with "same as Input"
/// when the output path matches the input path.
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

/// Moves `src` to `dst` (removing a previous `dst` if it exists, like ffmpeg
/// with `-y`). Does nothing when both paths are already the same.
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

/// Converts an arbitrary file name into a Windows-valid one: replaces the
/// forbidden characters (`< > : " / \ | ? *`) and control characters with `_`,
/// trims trailing spaces/dots (forbidden on Windows) and avoids the reserved
/// system names (CON, PRN, AUX, NUL, COM1-9, LPT1-9).
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

    // Windows does not allow trailing spaces or dots in the name.
    while out.ends_with(' ') || out.ends_with('.') {
        out.pop();
    }

    // Avoids reserved device names.
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

pub(crate) fn video_codec(ext: &str) -> &'static str {
    match ext {
        "webm" => "libvpx-vp9",
        "avi" => "mpeg4",
        _ => "libx264",
    }
}

pub(crate) fn audio_codec(ext: &str) -> &'static str {
    match ext {
        "mp3" => "libmp3lame",
        "webm" | "opus" => "libopus",
        "wav" => "pcm_s16le",
        _ => "aac",
    }
}

pub(crate) fn binary_path(app: &tauri::AppHandle, name: &str) -> Result<PathBuf, String> {
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

    let output = hidden_cmd(yt_dlp)
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

    // With --no-playlist, a "video + playlist" link returns only the video.
    // But a full playlist link makes yt-dlp return the playlist JSON (not
    // deserializable as VideoInfo): give a clear message in that case.
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

/// Starts the download on a separate thread and immediately returns an `id`
/// the frontend uses to follow progress via `download://progress` events.
/// This lets multiple downloads run in parallel without blocking the UI.
#[tauri::command]
pub fn download(
    app: tauri::AppHandle,
    url: String,
    folder: String,
    file_name: String,
    video_format_id: Option<String>,
    audio_format_id: Option<String>,
    video_ext: String,
    audio_ext: String,
    merge: bool,
    output_format: String,
    duration: Option<f64>,
) -> Result<String, String> {
    let dir = PathBuf::from(&folder);

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

    // Register the download as active so it can be cancelled by id.
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
            folder,
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

/// Cancels an active download: kills the process (yt-dlp/ffmpeg) and the
/// thread deletes the files downloaded so far.
#[tauri::command]
pub fn cancel_download(id: String) {
    if let Some(handle) = active().lock().unwrap().remove(&id) {
        handle.flag.store(true, Ordering::SeqCst);
        if let Some(pid) = handle.pid.lock().unwrap().take() {
            kill_tree(pid);
        }
    }
}

/// Name of the event emitted to the frontend with the update progress.
const UPDATE_EVENT: &str = "update://progress";

fn emit_update(app: &tauri::AppHandle, progress: f64, message: Option<&str>, error: Option<&str>) {
    let _ = app.emit(
        UPDATE_EVENT,
        UpdateProgress {
            progress,
            message: message.map(|s| s.to_string()),
            error: error.map(|s| s.to_string()),
        },
    );
}

/// Updates yt-dlp in place by running `yt-dlp -U`.
///
/// The app runs as administrator (the `requireAdministrator` manifest in
/// build.rs), so the binary bundled in the install path can be overwritten
/// without extra elevation. The command blocks until it finishes (it runs on
/// Tauri's command thread, not the UI) and keeps emitting the real progress
/// via `update://progress`: yt-dlp's own updater reuses the downloader and
/// prints `[download] xx.x%` lines to stdout.
#[tauri::command]
pub fn update_ytdlp(app: tauri::AppHandle) -> Result<(), String> {
    let yt_dlp = binary_path(&app, "yt-dlp")?;

    emit_update(&app, 0.0, Some("Comprobando actualización…"), None);

    let mut child = hidden_cmd(yt_dlp)
        .arg("-U")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("No se pudo lanzar yt-dlp -U: {e}"))?;

    // stderr (errors) is drained on a separate thread so nothing blocks, same
    // as downloads: if it is not drained the buffer fills up and yt-dlp hangs.
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

    // stdout (progress from the updater's downloader) is read live.
    let stdout = child.stdout.take().expect("stdout piped");
    for line in BufReader::new(stdout).lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if let Some(pct) = parse_percent(&line) {
            emit_update(&app, pct, Some("Descargando actualización…"), None);
        }
    }

    let status = child.wait().map_err(|e| e.to_string())?;
    let err_buf = err_handle.join().unwrap_or_default();

    if !status.success() {
        let msg = if err_buf.trim().is_empty() {
            "yt-dlp -U falló".to_string()
        } else {
            err_buf.trim().to_string()
        };
        emit_update(&app, 0.0, None, Some(&msg));
        return Err(msg);
    }

    // "already up to date" comes out through this path without `[download]`
    // lines: still emit 100% and the splash navigates to /home.
    emit_update(&app, 100.0, Some("yt-dlp actualizado"), None);
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_download(
    app: &tauri::AppHandle,
    id: &str,
    url: String,
    folder: String,
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
        folder.clone(),
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

    // If the download was cancelled, delete the files downloaded so far
    // (.part partials, already-downloaded video/audio, etc.).
    if cancel.flag.load(Ordering::SeqCst) {
        cleanup_download_files(Path::new(&folder), id);
    }

    result
}

/// Deletes every temp file of a cancelled download: any file starting with
/// `{id}_video` or `{id}_audio` (including yt-dlp `.part` partials).
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
    folder: String,
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
    let dir = PathBuf::from(&folder);

    // The final name is sanitized to be Windows-valid (the user may type `:`
    // or other forbidden characters that would break paths).
    let safe_name = sanitize_windows_filename(&file_name);
    // Temp files use the id as a neutral internal name: yt-dlp never modifies
    // it (unlike the user name, which it sanitizes on its own), so we always
    // find them; two simultaneous downloads with the same file name also never
    // collide.
    let video_out = format!("{id}_video.%(ext)s");
    let audio_out = format!("{id}_audio.%(ext)s");

    let mut video_file: Option<PathBuf> = None;
    let mut audio_file: Option<PathBuf> = None;

    // Video download: goes from 0% to 40% (or 70% when there is no audio).
    if has_video {
        let vid = video_format_id.unwrap();
        emit(
            app,
            id,
            DownloadStatus::Downloading,
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

    // Audio download: from 40% to 70%.
    if has_audio {
        let aud = audio_format_id.unwrap();
        emit(
            app,
            id,
            DownloadStatus::Downloading,
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
                emit(
                    app,
                    id,
                    DownloadStatus::Merging,
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
                    DownloadStatus::Merging,
                    (70.0, 99.0),
                    duration,
                    cancel,
                )?;
                let _ = std::fs::remove_file(vf);
                let _ = std::fs::remove_file(af);
            } else {
                // Separate video: only converted when the downloaded extension
                // is not already the requested one.
                let video_final = dir.join(format!("{safe_name}_video.{video_ext}"));
                if needs_conversion(vf, &video_final) {
                    emit(
                        app,
                        id,
                        DownloadStatus::Converting,
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
                        DownloadStatus::Converting,
                        (70.0, 84.5),
                        duration,
                        cancel,
                    )?;
                    let _ = std::fs::remove_file(vf);
                } else {
                    // Already in .{video_ext}: no ffmpeg needed.
                    rename_if_needed(vf, &video_final)?;
                }

                // Separate audio.
                let audio_final = dir.join(format!("{safe_name}_audio.{audio_ext}"));
                if needs_conversion(af, &audio_final) {
                    emit(
                        app,
                        id,
                        DownloadStatus::Converting,
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
                        DownloadStatus::Converting,
                        (84.5, 99.0),
                        duration,
                        cancel,
                    )?;
                    let _ = std::fs::remove_file(af);
                } else {
                    // Already in .{audio_ext}: no ffmpeg needed.
                    rename_if_needed(af, &audio_final)?;
                }
            }
        }
        (Some(vf), None) => {
            // Video only: if the downloaded file is already in the requested
            // extension, ffmpeg is not needed (just rename, or nothing if it
            // already has the final name). This avoids ffmpeg's "same as
            // Input" failure when input and output are the same file.
            let (target, codec) = if merge {
                (
                    dir.join(format!("{safe_name}.{output_format}")),
                    video_codec(&output_format),
                )
            } else {
                (
                    dir.join(format!("{safe_name}_video.{video_ext}")),
                    video_codec(&video_ext),
                )
            };
            if needs_conversion(vf, &target) {
                emit(
                    app,
                    id,
                    DownloadStatus::Converting,
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
                    DownloadStatus::Converting,
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
            // Audio only: same logic as the video-only case.
            let (target, codec) = if merge {
                (
                    dir.join(format!("{safe_name}.{output_format}")),
                    audio_codec(&output_format),
                )
            } else {
                (
                    dir.join(format!("{safe_name}_audio.{audio_ext}")),
                    audio_codec(&audio_ext),
                )
            };
            if needs_conversion(af, &target) {
                emit(
                    app,
                    id,
                    DownloadStatus::Converting,
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
                    DownloadStatus::Converting,
                    (70.0, 99.0),
                    duration,
                    cancel,
                )?;
                let _ = std::fs::remove_file(af);
            } else {
                rename_if_needed(af, &target)?;
            }
        }
        (None, None) => unreachable!("already validated: at least one format"),
    }

    emit(
        app,
        id,
        DownloadStatus::Completed,
        100.0,
        Some("Completado"),
        None,
    );
    Ok(())
}
