<script lang="ts">
  interface Props {
    /** Muestra u oculta el overlay. */
    open: boolean;
    /** Etiqueta accesible del overlay. */
    ariaLabel?: string;
  }

  let { open = false, ariaLabel = "Procesando solicitud" }: Props = $props();

  // Misma geometría que ProgressRing (viewBox 120)
  const R = 52;
  const CIRC = 2 * Math.PI * R;
  const ARC = CIRC * 0.28; // arco del círculo incompleto (~28%)
  // id único para el gradiente, para no colisionar si hay varios overlays
  const gradId = `spin-grad-${crypto.randomUUID()}`;
</script>

{#if open}
  <div class="overlay" role="status" aria-label={ariaLabel}>
    <div class="spinner-card" aria-hidden="true">
      <svg class="spinner" viewBox="0 0 120 120">
        <defs>
          <linearGradient id={gradId} x1="0%" y1="0%" x2="100%" y2="100%">
            <stop offset="0%" stop-color="var(--primary-dim)" />
            <stop offset="100%" stop-color="var(--primary)" />
          </linearGradient>
        </defs>
        <circle class="spinner-track" cx="60" cy="60" r={R} />
        <circle
          class="spinner-arc"
          cx="60"
          cy="60"
          r={R}
          stroke-dasharray={`${ARC} ${CIRC - ARC}`}
          transform="rotate(-90 60 60)"
          style={`stroke: url(#${gradId})`}
        />
      </svg>
    </div>
  </div>
{/if}

<style>
  /* El backdrop (fondo translúcido que tapa lo de abajo) usa el `.overlay` global
     de app.css: rgba oscuro + blur + fade-in + z-index 100. */

  .spinner-card {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 160px;
    height: 160px;
    background: var(--raised-surface);
    border: 1px solid var(--glass-edge);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-raised), var(--glow-primary);
    animation: card-in 180ms ease;
  }

  .spinner {
    display: block;
    width: 64px;
    height: 64px;
    animation: spin 900ms linear infinite;
  }

  .spinner-track {
    fill: none;
    stroke: var(--glass-edge);
    stroke-width: 6;
  }

  .spinner-arc {
    fill: none;
    stroke-width: 6;
    stroke-linecap: round;
    filter: drop-shadow(0 0 6px rgba(157, 92, 255, 0.3));
  }

  @keyframes spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }

  @keyframes card-in {
    from {
      opacity: 0;
      transform: scale(0.92);
    }
    to {
      opacity: 1;
      transform: none;
    }
  }
</style>
