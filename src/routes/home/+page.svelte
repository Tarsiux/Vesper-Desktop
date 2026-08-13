<script lang="ts">
  import Topbar from "$lib/components/Topbar.svelte";
  import Options from "$lib/components/Options.svelte";
  import LoadingOverlay from "$lib/components/LoadingOverlay.svelte";
  import type { DownloadOptions, Format, VideoInfo } from "$lib/types";
  import { invoke } from "@tauri-apps/api/core";

  let carpeta = "";
  let url = "";
  let info: VideoInfo | null = null;
  let videoFormats: Format[] = [];
  let audioFormats: Format[] = [];
  let optionsOpen = false;
  let loading = false;
  let error = "";

  async function select_folder() {
    try {
      const res = await invoke<string | null>("select_folder");
      if (res) {
        carpeta = res;
      }
    } catch (error) {
      console.error("Error al seleccionar la carpeta:", error);
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
      await invoke("descargar", { url, carpeta, ...opts });
      console.log("Descarga completada correctamente");
    } catch (e) {
      error = String(e);
      console.error("Error al descargar:", e);
    }
  }
</script>

<div class="page">
  <Topbar />

  <main class="container">
    <header class="page-header">
      <h1 class="headline-lg">Descargar</h1>
    </header>

    <form class="card download-card" on:submit|preventDefault={descargar}>
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
          <button type="button" class="btn btn--ghost" on:click={select_folder}>
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

      {#if error}
        <div class="error-banner" role="alert">
          <span class="error-dot" aria-hidden="true"></span>
          <p class="body-md">{error}</p>
        </div>
      {/if}

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

  <LoadingOverlay open={loading} />
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
</style>
