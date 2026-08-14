use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::Emitter;

use super::download::{binary_path, hidden_cmd, kill_tree, video_codec};
use crate::structs::{EditProgress, EditStatus};

/// Name of the event emitted to the frontend with the trim progress.
const EVENT: &str = "editor://progress";

/// Counter so trim ids stay unique.
static COUNTER: AtomicU64 = AtomicU64::new(0);

/// Identifies an active trim so it can be cancelled: a cancellation flag and
/// the pid of the currently running ffmpeg process.
#[derive(Clone)]
struct CancelHandle {
    flag: Arc<AtomicBool>,
    pid: Arc<Mutex<Option<u32>>>,
}

/// Active trims, keyed by id.
static ACTIVE: OnceLock<Mutex<HashMap<String, CancelHandle>>> = OnceLock::new();

fn active() -> &'static Mutex<HashMap<String, CancelHandle>> {
    ACTIVE.get_or_init(|| Mutex::new(HashMap::new()))
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
    status: EditStatus,
    progress: f64,
    message: Option<&str>,
    error: Option<&str>,
) {
    let _ = app.emit(
        EVENT,
        EditProgress {
            id: id.to_string(),
            status,
            progress,
            message: message.map(|s| s.to_string()),
            error: error.map(|s| s.to_string()),
        },
    );
}

/// Opens the native dialog to pick a video or audio file depending on the mode.
#[tauri::command]
pub fn select_media_file(kind: String) -> Result<Option<String>, String> {
    let (title, filters): (&str, Vec<(&str, Vec<&str>)>) = match kind.as_str() {
        "audio" => (
            "Seleccionar audio",
            vec![("Audio", vec!["mp3", "m4a", "wav", "flac", "ogg"])],
        ),
        _ => (
            "Seleccionar video",
            vec![("Video", vec!["mp4", "mkv", "webm", "mov", "avi"])],
        ),
    };

    let mut dialog = rfd::FileDialog::new().set_title(title);
    for (name, exts) in filters {
        dialog = dialog.add_filter(name, &exts);
    }

    Ok(dialog
        .pick_file()
        .map(|p| p.display().to_string().replace('\\', "/")))
}

/// Extracts `count` frames spread across the duration with ffmpeg and returns
/// them as JPEG data-URLs for the video timeline strip. One ffmpeg spawn per
/// frame, output to stdout (pipe) so nothing touches the disk; if a frame
/// fails (e.g. a DRM-protected file) it is skipped.
#[tauri::command]
pub fn generate_thumbnails(
    app: tauri::AppHandle,
    path: String,
    count: u32,
    duration: f64,
) -> Result<Vec<String>, String> {
    let ffmpeg = binary_path(&app, "ffmpeg")?;
    let count = count.clamp(1, 40);
    let duration = if duration.is_finite() && duration > 0.0 {
        duration
    } else {
        0.0
    };

    let mut thumbs = Vec::with_capacity(count as usize);
    for i in 0..count {
        // Timestamp of frame i, spread across the duration (center of its
        // slice so the edges are not hit).
        let t = if duration > 0.0 {
            (i as f64 + 0.5) * duration / count as f64
        } else {
            0.0
        };

        let output = hidden_cmd(&ffmpeg)
            .args([
                "-ss",
                &format!("{t:.3}"),
                "-i",
                &path,
                "-frames:v",
                "1",
                "-f",
                "mjpeg",
                "-vcodec",
                "mjpeg",
                "-",
            ])
            .output()
            .map_err(|e| e.to_string())?;

        if output.status.success() && !output.stdout.is_empty() {
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&output.stdout);
            thumbs.push(format!("data:image/jpeg;base64,{b64}"));
        }
    }
    Ok(thumbs)
}

/// Reads the file duration from `ffmpeg -i` output (the `Duration: HH:MM:SS.xx`
/// line in stderr). Used to map the real trim progress; if it cannot be read
/// the trim still works.
fn probe_duration(ffmpeg: &std::path::Path, path: &str) -> Option<f64> {
    let output = hidden_cmd(ffmpeg).arg("-i").arg(path).output().ok()?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    let line = stderr.lines().find(|l| l.trim_start().starts_with("Duration:"))?;
    let rest = line.split("Duration:").nth(1)?.trim();
    let time = rest.split(',').next()?.trim();
    let mut parts = time.split(':');
    let h: f64 = parts.next()?.parse().ok()?;
    let m: f64 = parts.next()?.parse().ok()?;
    let s: f64 = parts.next()?.parse().ok()?;
    Some(h * 3600.0 + m * 60.0 + s)
}

/// Starts the trim on a separate thread and immediately returns an `id` the
/// frontend uses to follow progress via `editor://progress` events (same
/// pattern as downloads).
///
/// - Audio: lossless cut with `-c copy` (instant).
/// - Video: re-encoded for a frame-exact cut (`-c:v` depending on the
///   container, `-c:a copy` to leave the audio untouched).
///
/// The result is written to a temp file in the same directory and then
/// overwrites the original file.
#[tauri::command]
pub fn trim_media(
    app: tauri::AppHandle,
    path: String,
    start: f64,
    end: f64,
    is_video: bool,
) -> Result<String, String> {
    let src = PathBuf::from(&path);
    if !src.is_file() {
        return Err("El archivo no existe".into());
    }
    if !start.is_finite() || !end.is_finite() || end <= start {
        return Err("Rango de recorte inválido".into());
    }

    let id = new_id();
    let thread_id = id.clone();

    let handle = CancelHandle {
        flag: Arc::new(AtomicBool::new(false)),
        pid: Arc::new(Mutex::new(None)),
    };
    active().lock().unwrap().insert(id.clone(), handle.clone());

    std::thread::spawn(move || {
        let result = run_trim(&app, &thread_id, path, start, end, is_video, &handle);
        active().lock().unwrap().remove(&thread_id);
        if let Err(e) = result {
            emit(&app, &thread_id, EditStatus::Error, 0.0, None, Some(&e));
        }
    });

    Ok(id)
}

#[allow(clippy::too_many_arguments)]
fn run_trim(
    app: &tauri::AppHandle,
    id: &str,
    path: String,
    start: f64,
    end: f64,
    is_video: bool,
    cancel: &CancelHandle,
) -> Result<(), String> {
    if cancel.flag.load(Ordering::SeqCst) {
        return Err("Recorte cancelado".into());
    }

    let ffmpeg = binary_path(app, "ffmpeg")?;
    let src = PathBuf::from(&path);

    // The temp file lives in the same directory (same volume) so the final
    // rename over the original is immediate.
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("mp4")
        .to_ascii_lowercase();
    let tmp = src.with_extension(format!("vesper-trim-{id}.{ext}"));
    let len = end - start;

    emit(app, id, EditStatus::Processing, 0.0, Some("Recortando…"), None);

    // Estimate the duration to map out_time_us -> % (if it fails, the trim
    // still works and the panel just stays in "processing" mode).
    let duration = probe_duration(&ffmpeg, &path);

    let mut args: Vec<String> = vec![
        "-y".into(),
        "-nostats".into(),
        "-progress".into(),
        "pipe:1".into(),
        "-ss".into(),
        format!("{start:.3}"),
        "-i".into(),
        path,
        "-t".into(),
        format!("{len:.3}"),
    ];
    if is_video {
        args.extend([
            "-c:v".into(),
            video_codec(&ext).into(),
            "-c:a".into(),
            "copy".into(),
        ]);
    } else {
        args.extend(["-c".into(), "copy".into()]);
    }
    args.push(tmp.to_string_lossy().to_string());

    let mut child = hidden_cmd(&ffmpeg)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;

    *cancel.pid.lock().unwrap() = Some(child.id());

    // If the cancellation arrived right during spawn, kill it here.
    if cancel.flag.load(Ordering::SeqCst) {
        kill_tree(child.id());
        let _ = child.wait();
        *cancel.pid.lock().unwrap() = None;
        let _ = std::fs::remove_file(&tmp);
        return Err("Recorte cancelado".into());
    }

    // stderr (errors) is drained on a separate thread so nothing blocks, same
    // as downloads: if it is not drained the buffer fills up and ffmpeg hangs.
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

    // stdout (`-progress pipe:1`) is read here, live.
    let stdout = child.stdout.take().expect("stdout piped");
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
            emit(
                app,
                id,
                EditStatus::Processing,
                frac * 100.0,
                Some("Recortando…"),
                None,
            );
        }
    }

    let status = child.wait().map_err(|e| e.to_string())?;
    *cancel.pid.lock().unwrap() = None;
    let err_buf = err_handle.join().unwrap_or_default();

    if cancel.flag.load(Ordering::SeqCst) {
        let _ = std::fs::remove_file(&tmp);
        return Err("Recorte cancelado".into());
    }

    if !status.success() {
        let _ = std::fs::remove_file(&tmp);
        let msg = if err_buf.trim().is_empty() {
            "ffmpeg falló al recortar".to_string()
        } else {
            err_buf.trim().to_string()
        };
        return Err(msg);
    }

    if !tmp.is_file() {
        return Err("ffmpeg no generó el archivo recortado".into());
    }

    // Overwrites the original file with the trim result.
    let _ = std::fs::remove_file(&src);
    std::fs::rename(&tmp, &src)
        .map_err(|e| format!("No se pudo sobrescribir el archivo original: {e}"))?;

    emit(app, id, EditStatus::Completed, 100.0, Some("Completado"), None);
    Ok(())
}

/// Cancels an active trim: kills ffmpeg and the thread deletes the temp file.
#[tauri::command]
pub fn cancel_trim(id: String) {
    if let Some(handle) = active().lock().unwrap().remove(&id) {
        handle.flag.store(true, Ordering::SeqCst);
        if let Some(pid) = handle.pid.lock().unwrap().take() {
            kill_tree(pid);
        }
    }
}
