<script lang="ts">
  import Topbar from "$lib/components/Topbar.svelte";

  let tab = "video";

  let calidadVideo = "mejor disponible";
  let formatoVideo = "MP4";
  let miniatura = true;

  let formatoAudio = "MP3";
  let calidadAudio = "320kbps";
  let metadatos = true;
</script>

<Topbar />

<button type="button" onclick={() => (tab = "video")}>Video</button>
<button type="button" onclick={() => (tab = "audio")}>Audio</button>

{#if tab === "video"}
  <select bind:value={calidadVideo}>
    <option>Mejor disponible</option>
    <option>1080p</option>
    <option>720p</option>
    <option>480p</option>
  </select>

  <select bind:value={formatoVideo}>
    <option>MP4</option>
    <option>MKV</option>
    <option>WEBM</option>
  </select>

  <label>
    <input type="checkbox" bind:checked={miniatura} />
    Miniatura
  </label>
{:else}
  <select bind:value={formatoAudio}>
    <option>MP3</option>
    <option>M4A</option>
    <option>AAC</option>
    <option>FLAC</option>
    <option>OGG</option>
    <option>WAV</option>
  </select>

  <select bind:value={calidadAudio}>
    <option>320kbps</option>
    <option>256kbps</option>
    <option>192kbps</option>
    <option>128kbps</option>
  </select>

  <label>
    <input type="checkbox" bind:checked={metadatos} />
    Metadatos y carátula
  </label>
{/if}