<script lang="ts">
  interface Props {
    /** Current progress, from 0 to 100. */
    progress: number;
    /** "Processing" state: enables the glow pulse and the processing text. */
    processing?: boolean;
    /** Custom status text (defaults to "Cargando" / "Procesando"). */
    label?: string;
    /** Accessible label of the progressbar. */
    ariaLabel?: string;
  }

  let {
    progress,
    processing = false,
    label,
    ariaLabel = "Cargando aplicación",
  }: Props = $props();

  // Ring radius and its circumference (viewBox 120)
  const R = 52;
  const CIRC = 2 * Math.PI * R;
  // unique gradient id, to avoid collisions with multiple rings on the page
  const gradId = `ring-grad-${crypto.randomUUID()}`;

  const pct = $derived(Math.round(progress));
  const dashOffset = $derived(CIRC * (1 - progress / 100));
  const status = $derived(label ?? (processing ? "Procesando" : "Cargando"));
</script>

<div
  class="ring"
  class:processing
  role="progressbar"
  aria-valuemin={0}
  aria-valuemax={100}
  aria-valuenow={pct}
  aria-label={ariaLabel}
>
  <svg viewBox="0 0 120 120" aria-hidden="true">
    <defs>
      <linearGradient id={gradId} x1="0%" y1="0%" x2="100%" y2="100%">
        <stop offset="0%" stop-color="var(--primary-dim)" />
        <stop offset="100%" stop-color="var(--primary)" />
      </linearGradient>
    </defs>
    <circle class="track" cx="60" cy="60" r={R} />
    <circle
      class="fill"
      cx="60"
      cy="60"
      r={R}
      stroke-dasharray={CIRC}
      stroke-dashoffset={dashOffset}
      transform="rotate(-90 60 60)"
      style={`stroke: url(#${gradId})`}
    />
  </svg>

  <div class="center">
    <span class="pct">{pct}<span class="unit">%</span></span>
    <span class="label-sm status">{status}</span>
  </div>
</div>

<style>
  .ring {
    position: relative;
    width: 168px;
    height: 168px;
  }

  .ring svg {
    display: block;
    width: 100%;
    height: 100%;
  }

  .track {
    fill: none;
    stroke: var(--glass-edge);
    stroke-width: 6;
  }

  .fill {
    fill: none;
    stroke-width: 6;
    stroke-linecap: round;
    transition: stroke-dashoffset 200ms linear;
    filter: drop-shadow(0 0 6px rgba(157, 92, 255, 0.3));
  }

  .center {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: var(--space-1);
  }

  .pct {
    font-family: var(--font-display);
    font-size: 40px;
    font-weight: 700;
    line-height: 48px;
    letter-spacing: -0.02em;
    color: var(--on-surface);
  }

  .unit {
    font-size: 22px;
    font-weight: 600;
    color: var(--on-surface-variant);
  }

  .status {
    color: var(--on-surface-variant);
  }

  /* "Processing" state during the pause at 50% */
  .ring.processing .status {
    color: var(--primary);
  }

  .ring.processing .fill {
    animation: fill-pulse 1.2s ease-in-out infinite;
  }

  @keyframes fill-pulse {
    0%,
    100% {
      filter: drop-shadow(0 0 4px rgba(157, 92, 255, 0.2));
    }
    50% {
      filter: drop-shadow(0 0 14px rgba(157, 92, 255, 0.45));
    }
  }

  @media (max-width: 640px) {
    .ring {
      width: 144px;
      height: 144px;
    }

    .pct {
      font-size: 32px;
      line-height: 40px;
    }

    .unit {
      font-size: 18px;
    }
  }
</style>
