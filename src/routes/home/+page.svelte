<script lang="ts">
  import Topbar from "$lib/components/Topbar.svelte";

  let formato = "video";
  let carpeta = "";
</script>

<Topbar />

<input type="text" />

<label>
  Seleccionar carpeta
  <input
    type="file"
    webkitdirectory
    onchange={(e) => (carpeta = e.currentTarget.files?.[0]?.webkitRelativePath.split("/")[0] ?? "")}
  />
</label>
<p>Ruta: {carpeta}</p>

<button type="button" onclick={() => (formato = "video")}>Video</button>
<button type="button" onclick={() => (formato = "audio")}>Audio</button>
<button type="submit">Descargar</button>
