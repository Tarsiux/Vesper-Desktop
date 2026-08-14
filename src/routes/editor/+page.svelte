<script lang="ts">
  import { onMount, tick } from "svelte";
  import { convertFileSrc, invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import Topbar from "$lib/components/Topbar.svelte";
  import ErrorToast from "$lib/components/ErrorToast.svelte";
  import TrimTimeline from "$lib/components/TrimTimeline.svelte";
  import type { EditProgress } from "$lib/types";

  type Mode = "video" | "audio";

  let mode: Mode = $state("video");

  // Archivo cargado
  let filePath = $state("");
  let fileName = $state("");
  let mediaUrl = $state("");
  let duration = $state(0);
  let isVideo = $state(false);

  // Recorte
  let trimStart = $state(0);
  let trimEnd = $state(0);
  let currentTime = $state(0);
  let playing = $state(false);

  // Visuales de la línea de tiempo
  let thumbs: string[] = $state([]);
  let waveform: number[] = $state([]);
  let loadSeq = 0;

  // Procesamiento
  let processing = $state(false);
  let progress = $state(0);
  let procMessage = $state("Recortando…");

  let error = $state("");
  let notice = $state("");
  let confirmOpen = $state(false);

  let videoEl: HTMLVideoElement | undefined = $state();
  let unlisten: (() => void) | undefined;
  let jobId = "";

  const mediaEl = $derived(videoEl);
  const visualMode = $derived(isVideo ? "video" : "audio");
  // La línea de tiempo muestra fotogramas solo si el usuario eligió el editor
  // de video y el archivo es realmente un video; en el resto de casos, waveform.
  const timelineVisual = $derived(mode === "video" && isVideo ? "video" : "audio");
  const hasTrim = $derived(duration > 0 && trimEnd - trimStart < duration - 0.1);
  const canSave = $derived(!!filePath && !processing && hasTrim);

  onMount(() => {
    // `listen` solo existe dentro de la ventana de Tauri; en un navegador
    // normal (`pnpm dev`) no está el runtime y esta llamada lanzaría una
    // excepción, así que solo nos suscribimos cuando Tauri está disponible.
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      listen<EditProgress>("editor://progress", (event) => {
        const p = event.payload;
        if (p.id !== jobId) return;
        if (p.status === "completado") {
          processing = false;
          notice = "Archivo recortado y sobrescrito correctamente";
          recargarPreview();
        } else if (p.status === "error") {
          processing = false;
          const msg = p.error ?? "Error al recortar el archivo";
          if (msg.toLowerCase().includes("cancelado")) {
            notice = "Recorte cancelado";
          } else {
            error = msg;
          }
        } else {
          progress = p.progress;
          procMessage = p.message ?? "Recortando…";
        }
      })
        .then((fn) => (unlisten = fn))
        .catch((e) =>
          console.error("No se pudo suscribirse a los eventos del editor:", e)
        );
    }
    return () => unlisten?.();
  });

  function switchMode(m: Mode) {
    if (processing || m === mode) return;
    mode = m;
    limpiar();
  }

  function limpiar() {
    filePath = "";
    fileName = "";
    mediaUrl = "";
    duration = 0;
    isVideo = false;
    trimStart = 0;
    trimEnd = 0;
    currentTime = 0;
    playing = false;
    thumbs = [];
    waveform = [];
    notice = "";
    error = "";
  }

  async function selectFile() {
    if (processing) return;
    error = "";
    notice = "";
    try {
      const p = await invoke<string | null>("select_media_file", {
        tipo: mode,
      });
      if (!p) return;
      filePath = p;
      fileName = p.split("/").pop() ?? p;
      thumbs = [];
      waveform = [];
      trimStart = 0;
      trimEnd = 0;
      currentTime = 0;
      playing = false;
      // Vacía el src un tick para forzar la recarga con el archivo nuevo.
      mediaUrl = "";
      await tick();
      mediaUrl = convertFileSrc(p);
    } catch (e) {
      error = String(e);
      console.error("Error al seleccionar el archivo:", e);
    }
  }

  // El <video> sirve también para reproducir audio; videoWidth > 0 indica que
  // el archivo es realmente un video.
  function onMetadata() {
    const v = videoEl;
    if (!v) return;
    const d = v.duration;
    if (!isFinite(d) || d <= 0) return;
    isVideo = (v.videoWidth ?? 0) > 0;
    duration = d;
    trimStart = 0;
    trimEnd = d;
    currentTime = 0;
    const seq = ++loadSeq;
    cargarVisuales(seq, isVideo);
  }

  async function cargarVisuales(seq: number, esVideo: boolean) {
    const d = duration;
    if (mode === "video" && esVideo) {
      try {
        const res = await invoke<string[]>("generar_thumbnails", {
          path: filePath,
          count: 16,
          duration: d,
        });
        if (seq === loadSeq) thumbs = res;
      } catch (e) {
        console.error("No se pudieron generar las vistas previas:", e);
        if (seq === loadSeq) thumbs = [];
      }
    } else {
      const w = await calcularWaveform();
      if (seq === loadSeq) waveform = w;
    }
  }

  async function calcularWaveform(): Promise<number[]> {
    if (!mediaUrl) return [];
    try {
      const res = await fetch(mediaUrl);
      if (!res.ok) return [];
      const buf = await res.arrayBuffer();
      const Ctor =
        window.AudioContext ??
        (window as unknown as { webkitAudioContext?: typeof AudioContext })
          .webkitAudioContext;
      if (!Ctor) return [];
      const ctx = new Ctor();
      try {
        const audio = await ctx.decodeAudioData(buf);
        const data = audio.getChannelData(0);
        const BUCKETS = 220;
        const peaks: number[] = [];
        const step = Math.max(1, Math.floor(data.length / BUCKETS));
        for (let i = 0; i < BUCKETS; i++) {
          const startIdx = i * step;
          const endIdx = Math.min(data.length, startIdx + step);
          let max = 0;
          for (let j = startIdx; j < endIdx; j++) {
            const v = Math.abs(data[j]);
            if (v > max) max = v;
          }
          peaks.push(max);
        }
        return peaks;
      } finally {
        ctx.close();
      }
    } catch (e) {
      console.error("No se pudo calcular el waveform:", e);
      return [];
    }
  }

  function onTimeUpdate() {
    const el = mediaEl;
    if (!el) return;
    currentTime = el.currentTime;
    // La reproducción se detiene al llegar al final del recorte.
    if (playing && el.currentTime >= trimEnd - 0.05) {
      el.pause();
      playing = false;
      currentTime = trimEnd;
    }
  }

  function onEnded() {
    playing = false;
    currentTime = trimEnd;
  }

  function togglePlay() {
    const el = mediaEl;
    if (!el) return;
    if (playing) {
      el.pause();
      playing = false;
    } else {
      // Si el playhead quedó fuera del recorte, vuelve al inicio del mismo.
      if (el.currentTime >= trimEnd - 0.05 || el.currentTime < trimStart) {
        el.currentTime = trimStart;
      }
      el.play()
        .then(() => (playing = true))
        .catch((e) => console.error("No se pudo reproducir:", e));
    }
  }

  // Mantiene el elemento sincronizado con el playhead (arrastre/búsqueda),
  // siempre dentro de la zona recortada.
  $effect(() => {
    const t = currentTime;
    const el = mediaEl;
    if (el && Math.abs(el.currentTime - t) > 0.05) {
      el.currentTime = Math.min(trimEnd, Math.max(trimStart, t));
    }
  });

  function onScrubStart() {
    const el = mediaEl;
    if (el && !el.paused) {
      el.pause();
      playing = false;
    }
  }

  function restablecer() {
    if (processing) return;
    trimStart = 0;
    trimEnd = duration;
    const el = mediaEl;
    if (el) {
      el.currentTime = 0;
      currentTime = 0;
    }
    notice = "";
    error = "";
  }

  function guardar() {
    if (!canSave) return;
    confirmOpen = true;
  }

  async function confirmarGuardar() {
    confirmOpen = false;
    error = "";
    notice = "";
    processing = true;
    progress = 0;
    procMessage = "Recortando…";
    try {
      const id = await invoke<string>("recortar_media", {
        path: filePath,
        start: trimStart,
        end: trimEnd,
        esVideo: isVideo,
      });
      jobId = id;
    } catch (e) {
      processing = false;
      error = String(e);
      console.error("Error al lanzar el recorte:", e);
    }
  }

  async function cancelarRecorte() {
    try {
      await invoke("cancelar_recorte", { id: jobId });
    } catch (e) {
      console.error("Error al cancelar el recorte:", e);
    }
  }

  // Tras guardar, el archivo cambió en disco: recarga la previsualización.
  function recargarPreview() {
    mediaUrl = "";
    setTimeout(() => {
      mediaUrl = convertFileSrc(filePath);
    }, 50);
    trimStart = 0;
    trimEnd = duration;
    currentTime = 0;
    playing = false;
  }

  function formatTime(t: number) {
    if (!isFinite(t) || t < 0) t = 0;
    const total = Math.floor(t);
    const h = Math.floor(total / 3600);
    const m = Math.floor((total % 3600) / 60);
    const s = total % 60;
    const mm = String(m).padStart(2, "0");
    const ss = String(s).padStart(2, "0");
    return h > 0 ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
  }
</script>

<div class="page">
  <Topbar />

  <main class="container editor">
    <header class="page-header">
      <h1 class="headline-lg">Editar</h1>
      <p class="body-md text-muted">
        Recorta el inicio y el final de tus videos y audios y guarda el
        resultado sobrescribiendo el archivo original.
      </p>
    </header>

    <div class="segmented" role="tablist" aria-label="Tipo de archivo a editar">
      <button
        class="chip"
        class:chip--active={mode === "video"}
        type="button"
        role="tab"
        aria-selected={mode === "video"}
        onclick={() => switchMode("video")}
      >
        Video
      </button>
      <button
        class="chip"
        class:chip--active={mode === "audio"}
        type="button"
        role="tab"
        aria-selected={mode === "audio"}
        onclick={() => switchMode("audio")}
      >
        Audio
      </button>
    </div>

    {#if !filePath}
      <section class="card empty-state">
        <div class="empty-icon" aria-hidden="true">
          {#if mode === "video"}
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <rect x="2" y="4" width="14" height="16" rx="2" />
              <path d="m16 10 6-3v10l-6-3" stroke-linecap="round" stroke-linejoin="round" />
            </svg>
          {:else}
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path
                d="M9 18V5l12-2v13"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <circle cx="6" cy="18" r="3" />
              <circle cx="18" cy="16" r="3" />
            </svg>
          {/if}
        </div>
        <h2 class="headline-md">Recorta {mode === "video" ? "un video" : "un audio"}</h2>
        <p class="body-md text-muted">
          {mode === "video"
            ? "Selecciona un video, ajusta las asas para recortar el inicio y el final y guarda el resultado sobrescribiendo el archivo original."
            : "Selecciona un audio, ajusta las asas para recortar el inicio y el final y guarda el resultado sobrescribiendo el archivo original."}
        </p>
        <button class="btn btn--primary btn--lg" type="button" onclick={selectFile}>
          Seleccionar {mode === "video" ? "video" : "audio"}
        </button>
        <p class="label-sm text-muted">
          {mode === "video"
            ? "MP4 · MKV · WEBM · MOV · AVI"
            : "MP3 · M4A · WAV · FLAC · OGG"}
        </p>
      </section>
    {:else}
      <section class="card editor-card">
        <div class="row row--between editor-head">
          <div class="file-meta">
            <p class="body-md text-truncate file-name" title={fileName}>{fileName}</p>
            <p class="label-sm">
              {formatTime(duration)} · {visualMode === "video" ? "Video" : "Audio"}
            </p>
          </div>
          <button
            class="btn btn--ghost btn--sm"
            type="button"
            onclick={selectFile}
            disabled={processing}
          >
            Cambiar archivo
          </button>
        </div>

        <!-- El elemento <video> se mantiene montado aunque sea audio (display
             none): así siempre carga los metadatos y detectamos el tipo. -->
        <div class="preview" class:preview-hidden={!isVideo}>
          <video
            bind:this={videoEl}
            src={mediaUrl}
            onloadedmetadata={onMetadata}
            ontimeupdate={onTimeUpdate}
            onended={onEnded}
          >
            <track kind="captions" />
          </video>
        </div>
        {#if !isVideo && filePath}
          <div class="audio-art" aria-hidden="true">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path
                d="M9 18V5l12-2v13"
                stroke-linecap="round"
                stroke-linejoin="round"
              />
              <circle cx="6" cy="18" r="3" />
              <circle cx="18" cy="16" r="3" />
            </svg>
          </div>
        {/if}

        <div class="row controls">
          <button
            class="btn btn--icon btn--outline"
            type="button"
            onclick={togglePlay}
            disabled={processing || duration <= 0}
            aria-label={playing ? "Pausar" : "Reproducir"}
          >
            {#if playing}
              <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
                <rect x="6" y="4" width="4" height="16" rx="1" />
                <rect x="14" y="4" width="4" height="16" rx="1" />
              </svg>
            {:else}
              <svg viewBox="0 0 24 24" fill="currentColor" aria-hidden="true">
                <path d="M7 4.5v15a1 1 0 0 0 1.53.85l12-7.5a1 1 0 0 0 0-1.7l-12-7.5A1 1 0 0 0 7 4.5Z" />
              </svg>
            {/if}
          </button>
          <span class="label-sm">{formatTime(currentTime)}</span>
          {#if !hasTrim}
            <span class="label-sm text-muted">— sin recorte aplicado</span>
          {/if}
        </div>

        <TrimTimeline
          mode={timelineVisual}
          {duration}
          bind:start={trimStart}
          bind:end={trimEnd}
          bind:currentTime={currentTime}
          {thumbs}
          {waveform}
          disabled={processing}
          onScrubStart={onScrubStart}
        />

        <div class="row row--end actions">
          <button
            class="btn btn--outline"
            type="button"
            onclick={restablecer}
            disabled={processing || !hasTrim}
          >
            Restablecer
          </button>
          <button class="btn btn--primary" type="button" onclick={guardar} disabled={!canSave}>
            Guardar
          </button>
        </div>
      </section>
    {/if}
  </main>

  {#if processing}
    <div class="processing-panel card" role="status" aria-live="polite">
      <div class="row row--between">
        <p class="body-md">{procMessage}</p>
        <span class="label-sm">{Math.round(progress)}%</span>
      </div>
      <div class="progress progress--pulse">
        <div class="progress__fill" style={`width: ${progress}%`}></div>
      </div>
      <button class="btn btn--ghost btn--sm" type="button" onclick={cancelarRecorte}>
        Cancelar
      </button>
    </div>
  {/if}

  {#if confirmOpen}
    <div
      class="overlay"
      role="presentation"
      onclick={(e) => {
        if (e.target === e.currentTarget) confirmOpen = false;
      }}
    >
      <div
        class="modal modal--sm"
        role="dialog"
        aria-modal="true"
        aria-labelledby="confirm-title"
      >
        <div class="modal-header">
          <h2 class="headline-md modal-title" id="confirm-title">Sobrescribir archivo</h2>
          <button
            class="modal-close"
            type="button"
            onclick={() => (confirmOpen = false)}
            aria-label="Cerrar"
          >
            &times;
          </button>
        </div>
        <div class="modal-body">
          <p class="body-md">
            Se va a guardar el recorte <strong>sobrescribiendo</strong> el
            archivo original <strong>{fileName}</strong>. Esta acción no se puede
            deshacer.
          </p>
        </div>
        <div class="modal-footer">
          <button
            class="btn btn--ghost"
            type="button"
            onclick={() => (confirmOpen = false)}
          >
            Cancelar
          </button>
          <button class="btn btn--primary" type="button" onclick={confirmarGuardar}>
            Sobrescribir
          </button>
        </div>
      </div>
    </div>
  {/if}

  {#if notice}
    <div class="notice-toast" role="status">
      <span class="notice-dot" aria-hidden="true"></span>
      <p class="body-md notice-msg">{notice}</p>
      <button class="modal-close" type="button" onclick={() => (notice = "")} aria-label="Cerrar">
        &times;
      </button>
    </div>
  {/if}

  <ErrorToast message={error} onDismiss={() => (error = "")} />
</div>

<style>
  .page {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
  }

  .page-header {
    margin-bottom: var(--space-4);
  }

  .editor .segmented {
    margin-bottom: var(--space-4);
  }

  /* Estado vacío: antes de elegir archivo */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: var(--space-3);
    padding: var(--space-6) var(--space-5);
  }

  .empty-state .headline-md {
    margin-top: var(--space-2);
  }

  .empty-state p {
    max-width: 520px;
  }

  .empty-icon {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 72px;
    height: 72px;
    border-radius: var(--radius-full);
    background: var(--active-fill);
    border: 1px solid var(--glass-edge);
    box-shadow: var(--glow-primary);
    color: var(--primary);
  }

  .empty-icon svg {
    width: 36px;
    height: 36px;
  }

  /* Tarjeta del editor */
  .editor-card {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-5);
  }

  .editor-head {
    gap: var(--space-4);
  }

  .file-meta {
    min-width: 0;
  }

  .file-name {
    max-width: 480px;
  }

  .preview {
    border-radius: var(--radius);
    overflow: hidden;
    background: #000;
    border: 1px solid var(--glass-edge);
  }

  .preview-hidden {
    display: none;
  }

  .preview video {
    display: block;
    width: 100%;
    max-height: 46vh;
    object-fit: contain;
    background: #000;
  }

  .audio-art {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 132px;
    border: 1px solid var(--glass-edge);
    border-radius: var(--radius);
    background:
      radial-gradient(420px 160px at 50% 0%, rgba(108, 71, 255, 0.14), transparent 70%),
      var(--surface-low);
    color: var(--primary);
  }

  .audio-art svg {
    width: 44px;
    height: 44px;
    opacity: 0.85;
  }

  .controls {
    gap: var(--space-3);
  }

  .controls .btn--icon {
    width: 42px;
    height: 42px;
    padding: 0;
    border-radius: var(--radius-full);
  }

  .controls .btn--icon svg {
    width: 20px;
    height: 20px;
  }

  .actions {
    gap: var(--space-2);
    margin-top: var(--space-2);
  }

  /* Panel de progreso mientras se recorta */
  .processing-panel {
    position: fixed;
    left: 50%;
    bottom: var(--space-5);
    transform: translateX(-50%);
    z-index: 90;
    width: min(480px, calc(100vw - var(--space-5)));
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-4);
  }

  .modal--sm {
    width: min(480px, 100%);
  }

  /* Aviso de éxito */
  .notice-toast {
    position: fixed;
    top: var(--space-4);
    left: 50%;
    transform: translateX(-50%);
    z-index: 200;
    width: min(92vw, 600px);
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 12px 16px;
    border: 1px solid rgba(157, 92, 255, 0.35);
    border-radius: var(--radius);
    background: rgba(30, 27, 46, 0.92);
    color: var(--on-surface);
    box-shadow: var(--shadow-raised), var(--glow-primary);
    backdrop-filter: blur(8px);
    -webkit-backdrop-filter: blur(8px);
  }

  .notice-dot {
    flex-shrink: 0;
    width: 8px;
    height: 8px;
    margin-top: 8px;
    border-radius: var(--radius-full);
    background: var(--primary);
    box-shadow: 0 0 8px rgba(157, 92, 255, 0.7);
  }

  .notice-msg {
    flex: 1;
    min-width: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>
