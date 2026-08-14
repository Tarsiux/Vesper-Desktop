<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import Topbar from "$lib/components/Topbar.svelte";
  import Options from "$lib/components/Options.svelte";
  import LoadingOverlay from "$lib/components/LoadingOverlay.svelte";
  import DownloadItem from "$lib/components/DownloadItem.svelte";
  import ErrorToast from "$lib/components/ErrorToast.svelte";
  import type {
    DownloadOptions,
    DownloadProgress,
    DownloadStatus,
    Format,
    VideoInfo,
  } from "$lib/types";

  interface QueueItem {
    id: string;
    fileName: string;
    status: DownloadStatus;
    progress: number;
    message: string | null;
    error: string | null;
  }

  let folder = $state("");
  let url = $state("");
  let info: VideoInfo | null = $state(null);
  let videoFormats: Format[] = $state([]);
  let audioFormats: Format[] = $state([]);
  let optionsOpen = $state(false);
  let loading = $state(false);
  let error = $state("");

  // Download queue: each entry is updated by `id` with the
  // `download://progress` events emitted by the backend from each download thread.
  let downloads: QueueItem[] = $state([]);
  let unlisten: (() => void) | undefined;

  onMount(() => {
    let disposed = false;

    // `listen` only exists inside the Tauri window. In a regular browser
    // (`pnpm dev`) the Tauri runtime is missing and this call would throw an
    // exception that breaks the page reactivity, so we only subscribe when the
    // Tauri runtime is available.
    if (typeof window !== "undefined" && "__TAURI_INTERNALS__" in window) {
      listen<DownloadProgress>("download://progress", (event) => {
        if (disposed) return;
        const p = event.payload;
        const idx = downloads.findIndex((d) => d.id === p.id);
        if (idx === -1) return;
        downloads[idx] = {
          ...downloads[idx],
          status: p.status,
          progress: p.progress,
          message: p.message ?? downloads[idx].message,
          error: p.error ?? downloads[idx].error,
        };
      })
        .then((fn) => {
          if (disposed) fn();
          else unlisten = fn;
        });
    }

    return () => {
      disposed = true;
      unlisten?.();
    };
  });

  // Removes the card from the queue and, if the download is still running,
  // asks the backend to cancel the process and delete the downloaded files.
  async function removeDownload(id: string) {
    downloads = downloads.filter((d) => d.id !== id);
    try {
      await invoke("cancel_download", { id });
    } catch (e) {
      error = String(e);
    }
  }

  async function selectFolder() {
    try {
      const res = await invoke<string | null>("select_folder");
      if (res) {
        folder = res;
      }
    } catch (e) {
      error = String(e);
    }
  }

  async function fetchVideoInfo() {
    error = "";
    loading = true;
    try {
      info = await invoke<VideoInfo>("show_options_video", { url });
      videoFormats = info.formats.filter(
        (f) => f.format_id && f.vcodec && f.vcodec !== "none"
      );
      audioFormats = info.formats.filter(
        (f) =>
          f.format_id &&
          f.acodec &&
          f.acodec !== "none" &&
          (!f.vcodec || f.vcodec === "none")
      );
      optionsOpen = true;
    } catch (e) {
      error = String(e);
    } finally {
      loading = false;
    }
  }

  async function handleDownload(opts: DownloadOptions) {
    error = "";
    if (!folder) {
      error = "Selecciona una carpeta de salida primero";
      return;
    }
    try {
      // The command no longer blocks: it starts the download on a thread and
      // returns the id.
      const id = await invoke<string>("download", {
        url,
        folder,
        ...opts,
        duration: info?.duration ?? null,
      });
      downloads.push({
        id,
        fileName: opts.fileName || info?.title || url,
        status: "downloading",
        progress: 0,
        message: "Descargando…",
        error: null,
      });
      optionsOpen = false;
      // Clears the URL field so the next link can be pasted.
      url = "";
    } catch (e) {
      error = String(e);
    }
  }
</script>

<div class="page">
  <Topbar />

  <main class="container">
    <header class="page-header">
      <h1 class="headline-lg">Descargar</h1>
    </header>

    <form
      class="card download-card"
      onsubmit={(e) => {
        e.preventDefault();
        fetchVideoInfo();
      }}
    >
      <div class="field">
        <label class="label-sm" for="url-input">URL del video</label>
        <input
          class="input"
          id="url-input"
          type="text"
          bind:value={url}
          placeholder="https://www.youtube.com/watch?v=..."
          autocomplete="off"
          spellcheck="false"
        />
      </div>

      <div class="field">
        <span class="label-sm">Carpeta de salida</span>
        <div class="row folder-row">
          <button
            type="button"
            class="btn btn--ghost"
            onclick={selectFolder}
          >
            Seleccionar carpeta
          </button>
          <span
            class="body-md text-truncate path"
            class:path-empty={!folder}
            title={folder}
          >
            {folder || "Ninguna carpeta seleccionada"}
          </span>
        </div>
      </div>

      <button type="submit" class="btn btn--primary btn--lg btn--block">
        Descargar
      </button>
    </form>
  </main>

  <Options
    bind:open={optionsOpen}
    {videoFormats}
    {audioFormats}
    title={info?.title ?? ""}
    onDownload={handleDownload}
  />

  {#if downloads.length > 0}
    <div class="container">
      <section class="queue">
        <header class="queue-header">
          <h2 class="headline-md">Descargas</h2>
          <span class="label-sm text-muted">
            {downloads.length}
            {downloads.length === 1 ? "descarga" : "descargas"} en cola
          </span>
        </header>
        <div class="queue-grid">
          {#each downloads as d (d.id)}
            <DownloadItem
              fileName={d.fileName}
              progress={d.progress}
              status={d.status}
              message={d.message}
              error={d.error}
              onRemove={() => removeDownload(d.id)}
            />
          {/each}
        </div>
      </section>
    </div>
  {/if}

  <LoadingOverlay open={loading} />

  <!-- Global errors (fetching info, folder, launching the download...). Errors
       of a specific download are shown on its own card. -->
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

  .download-card {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-5);
  }

  .folder-row {
    flex-wrap: wrap;
  }

  .folder-row .btn {
    flex-shrink: 0;
  }

  .path {
    flex: 1;
    min-width: 0;
  }

  .path-empty {
    color: var(--text-secondary);
    opacity: 0.75;
  }

  .queue {
    margin-top: var(--section-margin);
  }

  .queue-header {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    margin-bottom: var(--space-4);
  }

  .queue-grid {
    display: flex;
    flex-direction: column;
    gap: var(--card-gap);
  }
</style>
