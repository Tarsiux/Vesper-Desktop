<script lang="ts">
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import ProgressRing from "$lib/components/ProgressRing.svelte";
  import ErrorToast from "$lib/components/ErrorToast.svelte";
  import type { UpdateProgress } from "$lib/types";

  const TICK_MS = 35; // progress per step
  const HOLD_AT = 50; // percentage at which the yt-dlp update is launched
  const HOLD_MS = 900; // pause duration at 50% (only without the Tauri runtime)
  const FINAL_MS = 350; // short wait before navigating once at 100%

  // Are we inside the Tauri window? Without the runtime (`pnpm dev` in a
  // regular browser) there is no `invoke` or events, so the splash plays its
  // fake animation and still navigates.
  const isTauri =
    typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

  let progress = $state(0);
  let paused = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let holdTimer: ReturnType<typeof setTimeout> | undefined;

  // State of the yt-dlp update launched when reaching 50%.
  let updating = $state(false);
  let updatePct = $state<number | null>(null);
  let updateMsg = $state("");
  let error = $state("");
  let unlistenUpdate: (() => void) | undefined;
  let navigated = false;

  // While yt-dlp is updating, the ring shows the real progress sent by the
  // backend; without an update in progress it shows the fake animation progress.
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
        // In the real app: `yt-dlp -U` is launched and the splash waits until
        // it finishes (navigating when done, or after the error is dismissed).
        startUpdate();
      } else {
        // Without the Tauri runtime there is nothing to update: just the
        // decorative pause that already existed.
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

  // Runs `yt-dlp -U` (the app runs as administrator, so it can overwrite the
  // binary in the install path) and follows its real progress via the
  // `update://progress` event. On success it navigates to /home; on failure it
  // shows the error with ErrorToast and navigates when dismissed.
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

      await invoke("update_ytdlp");
      updatePct = 100;
      updateMsg = "yt-dlp actualizado";
      timer = setTimeout(() => finish(), FINAL_MS);
    } catch (e) {
      error = String(e);
      // The ErrorToast shows and, when dismissed (or expired), we navigate.
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

  /* Giant background logo, fixed to the screen, as a watermark */
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
    /* Square: never fills 100% of width or height, always fully visible and centered */
    width: min(88vw, 88vh);
    height: min(88vw, 88vh);
    object-fit: contain;
    opacity: 0.08;
    filter: saturate(0.55) brightness(1.05);
    user-select: none;
    -webkit-user-drag: none;
  }

  /* Loader content sits above the background */
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
