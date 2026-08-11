<script lang="ts">
  import Topbar from "$lib/components/Topbar.svelte";
  import { invoke } from "@tauri-apps/api/core";

  let formato = "video";
  let carpeta = "";

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


</script>

<Topbar />

<input type="text" />

<label>
  Seleccionar carpeta
  <input type="button" on:click={select_folder}/>
</label>
<p>Ruta: {carpeta}</p>

<button type="button" on:click={() => (formato = "video")}>Video</button>
<button type="button" on:click={() => (formato = "audio")}>Audio</button>
<button type="submit">Descargar</button>
