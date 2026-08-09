<script lang="ts">
  import Topbar from "$lib/components/Topbar.svelte";

  let formato = "video";
</script>

<Topbar />

<input type="text" />

<button type="button" onclick={() => (formato = "video")}>Video</button>
<button type="button" onclick={() => (formato = "audio")}>Audio</button>
<button type="submit">Descargar</button>
