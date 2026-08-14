<script lang="ts">
  import Select from "./Select.svelte";
  import type { DownloadOptions, Format } from "$lib/types";

  export let open = false;
  export let videoFormats: Format[] = [];
  export let audioFormats: Format[] = [];
  export let title = "";
  export let onDescargar: ((opts: DownloadOptions) => void) | undefined = undefined;

  let step: "video" | "audio" | "ajustes" = "video";
  let selectedVideo: string | null = null;
  let selectedAudio: string | null = null;

  function fmtId(f: Format, i: number): string {
    return f.format_id ?? String(i);
  }

  let fileName = "";
  let videoExt = "mp4";
  let audioExt = "mp3";
  let mergeAudioVideo = false;
  let outputFormat = "mp4";

  // Cada vez que se abre el modal se restablece la configuración inicial:
  // nunca debe arrastrar las opciones de la descarga anterior.
  $: if (open) {
    step = "video";
    selectedVideo = null;
    selectedAudio = null;
    fileName = "";
    videoExt = "mp4";
    audioExt = "mp3";
    mergeAudioVideo = false;
    outputFormat = "mp4";
  }

  // Solo se puede juntar si hay formatos de video y de audio seleccionados.
  const canMerge = () => !!selectedVideo && !!selectedAudio;

  // Si falta video o audio, el merge se apaga solo (las variables van
  // explícitas aquí para que Svelte las rastree).
  $: if (mergeAudioVideo && (!selectedVideo || !selectedAudio)) {
    mergeAudioVideo = false;
  }

  const videoExtensions = ["mp4", "mkv", "webm", "avi", "mov"];
  const audioExtensions = ["mp3", "m4a", "wav", "opus", "webm"];
  const outputFormats = ["mp4", "mkv", "webm", "avi", "mov"];

  function nextStep() {
    if (step === "video") step = "audio";
    else if (step === "audio") step = "ajustes";
  }

  function cerrar() {
    open = false;
  }

  function handleDescargar() {
    onDescargar?.({
      fileName: fileName || title,
      videoFormatId: selectedVideo,
      audioFormatId: selectedAudio,
      videoExt,
      audioExt,
      merge: mergeAudioVideo,
      outputFormat,
    });
  }

  function formatSize(bytes: number | null): string {
    if (!bytes) return "";
    const units = ["B", "KB", "MB", "GB"];
    let size = bytes;
    let unit = 0;
    while (size >= 1024 && unit < units.length - 1) {
      size /= 1024;
      unit++;
    }
    return `${size.toFixed(1)}${units[unit]}`;
  }
</script>

{#if open}
  <div class="overlay">
    <div class="modal">
      <header class="modal-header">
        <div class="modal-heading">
          <p class="label-sm text-muted">Opciones de descarga</p>
          <h2 class="headline-md modal-title">{title}</h2>
        </div>
        <button class="modal-close" type="button" on:click={cerrar} aria-label="Cerrar">
          &times;
        </button>
      </header>

      <div class="modal-tabs">
        <div class="segmented" role="tablist" aria-label="Pasos de descarga">
          <button
            class="chip"
            class:chip--active={step === "video"}
            type="button"
            on:click={() => (step = "video")}
          >
            Video
          </button>
          <button
            class="chip"
            class:chip--active={step === "audio"}
            type="button"
            on:click={() => (step = "audio")}
          >
            Audio
          </button>
          <button
            class="chip"
            class:chip--active={step === "ajustes"}
            type="button"
            on:click={() => (step = "ajustes")}
          >
            Ajustes
          </button>
        </div>
      </div>

      <div class="modal-body">
        {#if step === "video"}
          <table class="table">
            <thead>
              <tr>
                <th>Resolución</th>
                <th>Extensión</th>
                <th>Vcodec</th>
                <th>Audio</th>
              </tr>
            </thead>
            <tbody>
              <tr
                class="none-row"
                class:is-selected={selectedVideo === null}
                on:click={() => (selectedVideo = null)}
              >
                <td colspan="4" class="none-cell">
                  <span class="label-sm">Ninguno · solo audio</span>
                </td>
              </tr>
              {#each videoFormats as f, i (fmtId(f, i))}
                <tr
                  class:is-selected={selectedVideo === fmtId(f, i)}
                  on:click={() =>
                    (selectedVideo = selectedVideo === fmtId(f, i) ? null : fmtId(f, i))}
                >
                  <td>{f.resolution ?? "-"}</td>
                  <td>{f.ext ?? "-"}</td>
                  <td>{f.vcodec ?? "-"}</td>
                  <td>{f.acodec && f.acodec !== "none" ? "Sí" : "No"}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}

        {#if step === "audio"}
          <table class="table">
            <thead>
              <tr>
                <th>Calidad</th>
                <th>Extensión</th>
                <th>Codec</th>
                <th>Bitrate</th>
                <th>Tamaño</th>
              </tr>
            </thead>
            <tbody>
              <tr
                class="none-row"
                class:is-selected={selectedAudio === null}
                on:click={() => (selectedAudio = null)}
              >
                <td colspan="5" class="none-cell">
                  <span class="label-sm">Ninguno · solo video</span>
                </td>
              </tr>
              {#each audioFormats as f, i (fmtId(f, i))}
                <tr
                  class:is-selected={selectedAudio === fmtId(f, i)}
                  on:click={() =>
                    (selectedAudio = selectedAudio === fmtId(f, i) ? null : fmtId(f, i))}
                >
                  <td>{f.format_note ?? f.resolution ?? "-"}</td>
                  <td>{f.ext ?? "-"}</td>
                  <td>{f.acodec ?? "-"}</td>
                  <td>{f.tbr ? `${f.tbr.toFixed(0)}k` : "-"}</td>
                  <td>{formatSize(f.filesize)}</td>
                </tr>
              {/each}
            </tbody>
          </table>
        {/if}

        {#if step === "ajustes"}
          <div class="stack settings">
            <div class="field">
              <label class="label-sm" for="file-name">Nombre del archivo</label>
              <input
                class="input"
                id="file-name"
                type="text"
                value={fileName || title}
                on:input={(e) => (fileName = e.currentTarget.value)}
                placeholder={title}
              />
            </div>

            <div class="grid-2">
              <div class="card card--inset stack stack--sm">
                <h3 class="label-sm">Video</h3>
                <div class="field">
                  <span class="label-sm text-muted">Extensión final</span>
                  <Select options={videoExtensions} bind:value={videoExt} disabled={!selectedVideo} />
                </div>
              </div>
              <div class="card card--inset stack stack--sm">
                <h3 class="label-sm">Audio</h3>
                <div class="field">
                  <span class="label-sm text-muted">Extensión final</span>
                  <Select options={audioExtensions} bind:value={audioExt} disabled={!selectedAudio} />
                </div>
              </div>
            </div>

            <div>
              <button
                class="chip"
                class:chip--active={mergeAudioVideo}
                type="button"
                disabled={!canMerge()}
                on:click={() => (mergeAudioVideo = !mergeAudioVideo)}
              >
                Juntar audio y video
              </button>
            </div>

            {#if mergeAudioVideo}
              <div class="field">
                <span class="label-sm">Formato de salida del video final</span>
                <Select options={outputFormats} bind:value={outputFormat} />
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <footer class="modal-footer">
        <button class="btn btn--outline" type="button" on:click={cerrar}>Cerrar</button>
        {#if step !== "ajustes"}
          <button class="btn btn--primary" type="button" on:click={nextStep}>Siguiente</button>
        {:else}
          <button class="btn btn--primary" type="button" on:click={handleDescargar}>
            Descargar
          </button>
        {/if}
      </footer>
    </div>
  </div>
{/if}

<style>
  .modal-heading {
    min-width: 0;
  }

  .modal-tabs {
    padding: var(--space-3) var(--space-5) 0;
  }

  .settings {
    padding: var(--space-1) 0;
  }

  .none-cell {
    text-align: center;
    color: var(--text-secondary);
    cursor: pointer;
    user-select: none;
  }

  .none-row.is-selected .none-cell,
  .none-row.is-selected:hover .none-cell {
    color: var(--primary);
  }
</style>
