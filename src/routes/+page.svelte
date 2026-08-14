<script lang="ts">
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import ProgressRing from "$lib/components/ProgressRing.svelte";
  import ErrorToast from "$lib/components/ErrorToast.svelte";
  import type { UpdateProgress } from "$lib/types";

  const TICK_MS = 35; // avance por paso
  const HOLD_AT = 50; // porcentaje donde se lanza la actualización de yt-dlp
  const HOLD_MS = 900; // duración de la pausa en 50% (solo sin runtime de Tauri)
  const FINAL_MS = 350; // breve espera al llegar a 100%

  // ¿Estamos dentro de la ventana de Tauri? Sin el runtime (`pnpm dev` en un
  // navegador normal) no hay `invoke` ni eventos, así que el splash hace su
  // animación falsa y navega igualmente.
  const isTauri =
    typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

  let progress = $state(0);
  let paused = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let holdTimer: ReturnType<typeof setTimeout> | undefined;

  // Estado de la actualización de yt-dlp lanzada al llegar al 50%.
  let updating = $state(false);
  let updatePct = $state<number | null>(null);
  let updateMsg = $state("");
  let error = $state("");
  let unlistenUpdate: (() => void) | undefined;
  let navigated = false;

  // Mientras se actualiza yt-dlp, el anillo muestra el progreso real que manda
  // el backend; si no hay update en curso, el progreso falso de la animación.
  const displayProgress = $derived(updatePct ?? progress);

  function step() {
    progress = Math.min(100, progress + 1);

    if (progress >= 100) {
      timer = setTimeout(() => finish(), FINAL_MS);
      return;
    }

    if (progress === HOLD_AT && !paused) {
      paused = true;
      if (isTauri) {
        // En la app real: se lanza `yt-dlp -U` y el splash se queda esperando
        // a que termine (navega al acabar o si falla, tras cerrar el aviso).
        startUpdate();
      } else {
        // Sin runtime de Tauri no hay nada que actualizar: solo la pausa
        // decorativa que ya existía.
        holdTimer = setTimeout(() => {
          paused = false;
          timer = setTimeout(step, TICK_MS);
        }, HOLD_MS);
      }
      return;
    }

    timer = setTimeout(step, TICK_MS);
  }

  function finish() {
    if (navigated) return;
    navigated = true;
    goto("/home");
  }

  // Ejecuta `yt-dlp -U` (la app corre como administrador, así que puede
  // sobrescribir el binario en la ruta de instalación) y sigue su progreso
  // real vía el evento `update://progress`. Al terminar navega a /home; si
  // falla, muestra el error con el ErrorToast y navega al cerrarlo.
  async function startUpdate() {
    updating = true;
    updateMsg = "Comprobando actualización…";
    try {
      const unlisten = await listen<UpdateProgress>(
        "update://progress",
        (event) => {
          const p = event.payload;
          updatePct = p.progress;
          if (p.message) updateMsg = p.message;
          if (p.error) error = p.error;
        }
      );
      unlistenUpdate = unlisten;

      await invoke("actualizar_ytdlp");
      updatePct = 100;
      updateMsg = "yt-dlp actualizado";
      timer = setTimeout(() => finish(), FINAL_MS);
    } catch (e) {
      error = String(e);
      console.error("Error al actualizar yt-dlp:", e);
      // El ErrorToast se muestra y, al cerrarlo (o al expirar), se navega.
    } finally {
      updating = false;
      unlistenUpdate?.();
      unlistenUpdate = undefined;
    }
  }

  function dismissError() {
    error = "";
    timer = setTimeout(() => finish(), FINAL_MS);
  }

  onMount(() => {
    timer = setTimeout(step, TICK_MS);
    return () => {
      if (timer) clearTimeout(timer);
      if (holdTimer) clearTimeout(holdTimer);
      unlistenUpdate?.();
    };
  });
</script>

<div class="loader">
  <div class="logo-backdrop" aria-hidden="true">
    <img src="/logo.svg" alt="" />
  </div>

  <ProgressRing
    progress={displayProgress}
    processing={paused}
    label={updating ? updateMsg : undefined}
    ariaLabel={updating ? "Actualizando yt-dlp" : "Cargando aplicación"}
  />
</div>

<ErrorToast message={error} onDismiss={dismissError} />

<style>
  .loader {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-5);
    min-height: 100vh;
    padding: var(--container-padding);
    box-sizing: border-box;
    animation: loader-in 400ms ease;
  }

  /* Logo gigante de fondo, fijo a la pantalla, como marca de agua */
  .logo-backdrop {
    position: fixed;
    inset: 0;
    z-index: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    pointer-events: none;
  }

  .logo-backdrop img {
    /* Cuadrado: nunca ocupa el 100% de ancho ni alto, siempre se ve entero y centrado */
    width: min(88vw, 88vh);
    height: min(88vw, 88vh);
    object-fit: contain;
    opacity: 0.08;
    filter: saturate(0.55) brightness(1.05);
    user-select: none;
    -webkit-user-drag: none;
  }

  /* El contenido del loader queda por encima del fondo */
  .loader > :not(.logo-backdrop) {
    position: relative;
    z-index: 1;
  }

  @keyframes loader-in {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }
</style>
