use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::Manager;

use crate::structs::VideoInfo;

fn run(cmd: &Path, args: &[String], cwd: &Path) -> Result<(), String> {
    let output = Command::new(cmd)
        .current_dir(cwd)
        .args(args)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("{}{}", stdout.trim(), stderr.trim()));
    }

    Ok(())
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
        .arg(&url)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("yt-dlp falló: {}", stderr.trim()));
    }

    let info: VideoInfo = serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())?;

    Ok(info)
}

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
) -> Result<(), String> {
    let yt_dlp = binary_path(&app, "yt-dlp")?;
    let ffmpeg = binary_path(&app, "ffmpeg")?;
    let dir = PathBuf::from(&carpeta);

    if !dir.is_dir() {
        return Err("La carpeta de salida no existe".into());
    }

    let video_id = video_format_id.filter(|s| !s.trim().is_empty());
    let audio_id = audio_format_id.filter(|s| !s.trim().is_empty());

    if video_id.is_none() && audio_id.is_none() {
        return Err("Selecciona al menos un formato de video o de audio".into());
    }

    let video_out = format!("{file_name}_video.%(ext)s");
    let audio_out = format!("{file_name}_audio.%(ext)s");

    let mut video_file: Option<PathBuf> = None;
    let mut audio_file: Option<PathBuf> = None;

    if let Some(vid) = video_id {
        println!("Descargando video (formato {vid})...");
        run(
            &yt_dlp,
            &[
                "-f".into(),
                vid,
                "-o".into(),
                video_out,
                "-q".into(),
                "--no-warnings".into(),
                url.clone(),
            ],
            &dir,
        )?;
        video_file = Some(find_output(&dir, &format!("{file_name}_video"))?);
    }

    if let Some(aud) = audio_id {
        println!("Descargando audio (formato {aud})...");
        run(
            &yt_dlp,
            &[
                "-f".into(),
                aud,
                "-o".into(),
                audio_out,
                "-q".into(),
                "--no-warnings".into(),
                url,
            ],
            &dir,
        )?;
        audio_file = Some(find_output(&dir, &format!("{file_name}_audio"))?);
    }

    match (&video_file, &audio_file) {
        (Some(vf), Some(af)) => {
            if merge {
                let final_path = dir.join(format!("{file_name}.{output_format}"));
                println!("Juntando audio y video y convirtiendo a .{output_format}...");
                run(
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
                )?;
            } else {
                let video_final = dir.join(format!("{file_name}_video.{video_ext}"));
                let audio_final = dir.join(format!("{file_name}_audio.{audio_ext}"));
                println!("Convirtiendo video a .{video_ext}...");
                run(
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
                )?;
                println!("Convirtiendo audio a .{audio_ext}...");
                run(
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
                )?;
            }
        }
        (Some(vf), None) => {
            // Solo video: sin audio no hay nada que juntar, se convierte directamente.
            let (target, codec, label) = if merge {
                (
                    dir.join(format!("{file_name}.{output_format}")),
                    video_codec(&output_format),
                    format!(".{output_format}"),
                )
            } else {
                (
                    dir.join(format!("{file_name}_video.{video_ext}")),
                    video_codec(&video_ext),
                    format!(".{video_ext}"),
                )
            };
            println!("Convirtiendo video a {label}...");
            run(
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
            )?;
        }
        (None, Some(af)) => {
            // Solo audio: se convierte directamente.
            let (target, codec, label) = if merge {
                (
                    dir.join(format!("{file_name}.{output_format}")),
                    audio_codec(&output_format),
                    format!(".{output_format}"),
                )
            } else {
                (
                    dir.join(format!("{file_name}_audio.{audio_ext}")),
                    audio_codec(&audio_ext),
                    format!(".{audio_ext}"),
                )
            };
            println!("Convirtiendo audio a {label}...");
            run(
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
            )?;
        }
        (None, None) => unreachable!("ya validado: al menos un formato"),
    }

    if let Some(vf) = &video_file {
        let _ = std::fs::remove_file(vf);
    }
    if let Some(af) = &audio_file {
        let _ = std::fs::remove_file(af);
    }

    println!("Descarga completada");
    Ok(())
}
