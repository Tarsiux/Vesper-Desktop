<script lang="ts">
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";
  import ProgressRing from "$lib/components/ProgressRing.svelte";

  const TICK_MS = 35; // avance por paso
  const HOLD_AT = 50; // porcentaje donde se hace la pausa
  const HOLD_MS = 900; // duración de la pausa en 50%
  const FINAL_MS = 350; // breve espera al llegar a 100%

  let progress = $state(0);
  let paused = $state(false);
  let timer: ReturnType<typeof setTimeout> | undefined;
  let holdTimer: ReturnType<typeof setTimeout> | undefined;

  function step() {
    progress = Math.min(100, progress + 1);

    if (progress >= 100) {
      timer = setTimeout(() => goto("/home"), FINAL_MS);
      return;
    }

    if (progress === HOLD_AT && !paused) {
      paused = true;
      holdTimer = setTimeout(() => {
        paused = false;
        timer = setTimeout(step, TICK_MS);
      }, HOLD_MS);
      return;
    }

    timer = setTimeout(step, TICK_MS);
  }

  onMount(() => {
    timer = setTimeout(step, TICK_MS);
    return () => {
      if (timer) clearTimeout(timer);
      if (holdTimer) clearTimeout(holdTimer);
    };
  });
</script>

<div class="loader">
  <div class="logo-backdrop" aria-hidden="true">
    <img src="/logo.svg" alt="" />
  </div>

  <ProgressRing progress={progress} processing={paused} />
</div>

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
