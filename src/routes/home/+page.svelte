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

  interface ColaItem {
    id: string;
    fileName: string;
    status: DownloadStatus;
    progress: number;
    message: string | null;
    error: string | null;
  }

  let carpeta = $state("");
  let url = $state("");
  let info: VideoInfo | null = $state(null);
  let videoFormats: Format[] = $state([]);
  let audioFormats: Format[] = $state([]);
  let optionsOpen = $state(false);
  let loading = $state(false);
  let error = $state("");

  // Cola de descargas: cada entrada se actualiza por `id` con los eventos
  // `download://progress` que emite el backend desde cada hilo de descarga.
  let downloads: ColaItem[] = $state([]);
  let unlisten: (() => void) | undefined;

  onMount(() => {
    let disposed = false;

    // `listen` solo existe dentro de la ventana de Tauri. En un navegador
    // normal (`pnpm dev`) no está el runtime de Tauri y esta llamada lanza
    // una excepción que rompería la reactividad de la página, así que solo
    // nos suscribimos cuando el runtime de Tauri está disponible.
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
        })
        .catch((e) =>
          console.error("No se pudo suscribirse a los eventos de descarga:", e)
        );
    }

    return () => {
      disposed = true;
      unlisten?.();
    };
  });

  // Quita la tarjeta de la cola y, si la descarga sigue en curso, le pide al
  // backend que cancele el proceso y borre los archivos descargados.
  async function quitarDescarga(id: string) {
    downloads = downloads.filter((d) => d.id !== id);
    try {
      await invoke("cancelar_descarga", { id });
    } catch (e) {
      error = String(e);
      console.error("Error al cancelar la descarga:", e);
    }
  }

  async function select_folder() {
    try {
      const res = await invoke<string | null>("select_folder");
      if (res) {
        carpeta = res;
      }
    } catch (e) {
      error = String(e);
      console.error("Error al seleccionar la carpeta:", e);
    }
  }

  async function descargar() {
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
      console.error("Error al obtener opciones:", e);
    } finally {
      loading = false;
    }
  }

  async function handleDescargar(opts: DownloadOptions) {
    error = "";
    if (!carpeta) {
      error = "Selecciona una carpeta de salida primero";
      console.error(error);
      return;
    }
    try {
      // El comando ya no bloquea: lanza la descarga en un hilo y devuelve el id.
      const id = await invoke<string>("descargar", {
        url,
        carpeta,
        ...opts,
        duration: info?.duration ?? null,
      });
      downloads.push({
        id,
        fileName: opts.fileName || info?.title || url,
        status: "descargando",
        progress: 0,
        message: "Descargando…",
        error: null,
      });
      optionsOpen = false;
      // Limpia el campo de URL para poder pegar la siguiente.
      url = "";
    } catch (e) {
      error = String(e);
      console.error("Error al lanzar la descarga:", e);
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
        descargar();
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
            onclick={select_folder}
          >
            Seleccionar carpeta
          </button>
          <span
            class="body-md text-truncate path"
            class:path-empty={!carpeta}
            title={carpeta}
          >
            {carpeta || "Ninguna carpeta seleccionada"}
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
    onDescargar={handleDescargar}
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
              onQuitar={() => quitarDescarga(d.id)}
            />
          {/each}
        </div>
      </section>
    </div>
  {/if}

  <LoadingOverlay open={loading} />

  <!-- Errores globales (obtener info, carpeta, lanzar descarga...). Los errores
       de una descarga en concreto se muestran en su propia tarjeta. -->
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
