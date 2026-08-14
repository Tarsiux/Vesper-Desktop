<script lang="ts">
  interface Props {
    /** Tipo de visualización de la línea: fotogramas (video) o waveform (audio). */
    mode: "video" | "audio";
    /** Duración total del medio en segundos. */
    duration: number;
    /** Inicio del recorte (segundos), enlazable. */
    start: number;
    /** Fin del recorte (segundos), enlazable. */
    end: number;
    /** Posición actual del playhead (segundos), enlazable. */
    currentTime: number;
    /** Fotogramas (data-URLs) para la tira del video. */
    thumbs?: string[];
    /** Picos normalizados (0-1) para el waveform del audio. */
    waveform?: number[];
    disabled?: boolean;
    /** Se llama al empezar a arrastrar (asas o playhead) para pausar la reproducción. */
    onScrubStart?: () => void;
    /** Se llama al soltar el arrastre. */
    onScrubEnd?: () => void;
  }

  let {
    mode,
    duration,
    start = $bindable(),
    end = $bindable(),
    currentTime = $bindable(),
    thumbs = [],
    waveform = [],
    disabled = false,
    onScrubStart,
    onScrubEnd,
  }: Props = $props();

  let trackEl: HTMLDivElement | undefined = $state();
  let dragging: "start" | "end" | "playhead" | null = $state(null);

  // Separación mínima entre las asas: 0.25s o el 2% de la duración (lo menor).
  const MIN_GAP = $derived(Math.max(0.1, Math.min(0.25, duration * 0.02)));

  function pct(t: number) {
    if (duration <= 0) return 0;
    return (t / duration) * 100;
  }

  function clampTime(t: number) {
    return Math.min(duration, Math.max(0, t));
  }

  function timeFromEvent(e: PointerEvent) {
    const rect = trackEl!.getBoundingClientRect();
    const ratio = Math.min(1, Math.max(0, (e.clientX - rect.left) / rect.width));
    return ratio * duration;
  }

  function onTrackPointerDown(e: PointerEvent) {
    if (disabled || duration <= 0 || !trackEl) return;

    const t = timeFromEvent(e);
    const rect = trackEl.getBoundingClientRect();
    const startPx = rect.left + (pct(start) / 100) * trackEl.clientWidth;
    const endPx = rect.left + (pct(end) / 100) * trackEl.clientWidth;
    const TOLERANCE = 10; // px alrededor del centro del asa

    if (Math.abs(e.clientX - endPx) <= TOLERANCE) {
      dragging = "end";
    } else if (Math.abs(e.clientX - startPx) <= TOLERANCE) {
      dragging = "start";
    } else {
      dragging = "playhead";
    }

    trackEl.setPointerCapture(e.pointerId);
    onScrubStart?.();
    applyDrag(e);
  }

  function onTrackPointerMove(e: PointerEvent) {
    if (!dragging || disabled || !trackEl) return;
    applyDrag(e);
  }

  function onTrackPointerUp() {
    if (!dragging) return;
    dragging = null;
    onScrubEnd?.();
  }

  function applyDrag(e: PointerEvent) {
    const t = clampTime(timeFromEvent(e));

    if (dragging === "start") {
      start = Math.min(t, end - MIN_GAP);
    } else if (dragging === "end") {
      end = Math.max(t, start + MIN_GAP);
    } else if (dragging === "playhead") {
      // El playhead no sale del rango seleccionado.
      currentTime = Math.min(end, Math.max(start, t));
    }
  }

  // Accesibilidad: las asas también se pueden mover con las flechas.
  function nudge(which: "start" | "end", e: KeyboardEvent) {
    if (disabled) return;
    const step = Math.max(0.1, duration * 0.005);
    if (e.key === "ArrowLeft") {
      e.preventDefault();
      if (which === "start") start = Math.max(0, start - step);
      else end = Math.max(start + MIN_GAP, end - step);
    } else if (e.key === "ArrowRight") {
      e.preventDefault();
      if (which === "start") start = Math.min(end - MIN_GAP, start + step);
      else end = Math.min(duration, end + step);
    }
  }

  function formatTime(t: number) {
    if (!isFinite(t) || t < 0) t = 0;
    const total = Math.floor(t);
    const h = Math.floor(total / 3600);
    const m = Math.floor((total % 3600) / 60);
    const s = total % 60;
    const mm = String(m).padStart(2, "0");
    const ss = String(s).padStart(2, "0");
    return h > 0 ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
  }
</script>

<div class="timeline" class:disabled>
  <div
    class="track"
    bind:this={trackEl}
    role="group"
    aria-label="Línea de tiempo de recorte"
    onpointerdown={onTrackPointerDown}
    onpointermove={onTrackPointerMove}
    onpointerup={onTrackPointerUp}
    onpointercancel={onTrackPointerUp}
  >
    {#if mode === "video" && thumbs.length > 0}
      <div class="thumbs" aria-hidden="true">
        {#each thumbs as t, i (i)}
          <img class="thumb" src={t} alt="" draggable="false" />
        {/each}
      </div>
    {:else if mode === "audio" && waveform.length > 0}
      <div class="wave" aria-hidden="true">
        {#each waveform as peak, i (i)}
          <span class="bar" style={`height: ${Math.max(4, peak * 100)}%`}></span>
        {/each}
      </div>
    {:else}
      <div class="track-plain" aria-hidden="true"></div>
    {/if}

    <!-- Zonas descartadas del recorte -->
    <div class="dim dim--left" style={`width: ${pct(start)}%`} aria-hidden="true"></div>
    <div class="dim dim--right" style={`width: ${100 - pct(end)}%`} aria-hidden="true"></div>

    <!-- Zona seleccionada -->
    <div
      class="selection"
      style={`left: ${pct(start)}%; right: ${100 - pct(end)}%`}
      aria-hidden="true"
    ></div>

    <!-- Asa de inicio -->
    <div
      class="handle handle--start"
      style={`left: ${pct(start)}%`}
      role="slider"
      tabindex="0"
      aria-label="Inicio del recorte"
      aria-valuemin={0}
      aria-valuemax={duration}
      aria-valuenow={start}
      aria-valuetext={formatTime(start)}
      onkeydown={(e) => nudge("start", e)}
    ></div>

    <!-- Asa de fin -->
    <div
      class="handle handle--end"
      style={`left: ${pct(end)}%`}
      role="slider"
      tabindex="0"
      aria-label="Fin del recorte"
      aria-valuemin={0}
      aria-valuemax={duration}
      aria-valuenow={end}
      aria-valuetext={formatTime(end)}
      onkeydown={(e) => nudge("end", e)}
    ></div>

    <!-- Playhead -->
    <div class="playhead" style={`left: ${pct(currentTime)}%`} aria-hidden="true"></div>
  </div>

  <div class="times" aria-hidden="true">
    <span class="label-sm">{formatTime(start)}</span>
    <span class="label-sm">{formatTime(currentTime)}</span>
    <span class="label-sm">{formatTime(end)}</span>
  </div>
</div>

<style>
  .timeline {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .track {
    position: relative;
    height: 72px;
    border: 1px solid var(--glass-edge);
    border-radius: var(--radius-md);
    background: var(--surface-low);
    overflow: hidden;
    cursor: pointer;
    touch-action: none;
    user-select: none;
  }

  .timeline.disabled .track {
    cursor: not-allowed;
    opacity: 0.55;
  }

  /* Tira de fotogramas (video) */
  .thumbs {
    position: absolute;
    inset: 0;
    display: flex;
  }

  .thumb {
    flex: 1 1 0;
    min-width: 0;
    height: 100%;
    object-fit: cover;
  }

  /* Waveform (audio) */
  .wave {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    gap: 1px;
    padding: 0 4px;
  }

  .bar {
    flex: 1 1 0;
    min-width: 1px;
    border-radius: 1px;
    background: linear-gradient(180deg, var(--primary-dim), var(--primary));
    opacity: 0.55;
  }

  /* Fondo neutro cuando no hay fotogramas ni waveform */
  .track-plain {
    position: absolute;
    inset: 0;
    background: repeating-linear-gradient(
      90deg,
      var(--surface-high) 0 12px,
      var(--surface-mid) 12px 24px
    );
  }

  /* Partes descartadas del recorte */
  .dim {
    position: absolute;
    top: 0;
    bottom: 0;
    background: rgba(13, 12, 20, 0.66);
    pointer-events: none;
  }

  .dim--left {
    left: 0;
  }

  .dim--right {
    right: 0;
  }

  /* Zona seleccionada */
  .selection {
    position: absolute;
    top: 0;
    bottom: 0;
    background: rgba(157, 92, 255, 0.18);
    border-inline: 1px solid var(--primary);
    box-shadow: inset 0 0 12px rgba(157, 92, 255, 0.12);
    pointer-events: none;
  }

  /* Asas */
  .handle {
    position: absolute;
    top: 0;
    bottom: 0;
    z-index: 3;
    width: 14px;
    margin-left: -7px;
    background: var(--primary);
    cursor: ew-resize;
    touch-action: none;
  }

  .handle::before {
    content: "";
    position: absolute;
    left: 50%;
    top: 5px;
    bottom: 5px;
    width: 2px;
    transform: translateX(-50%);
    border-radius: 2px;
    background: rgba(255, 255, 255, 0.85);
  }

  .handle:focus-visible {
    outline: 2px solid var(--on-surface);
    outline-offset: -2px;
  }

  /* Playhead */
  .playhead {
    position: absolute;
    top: 0;
    bottom: 0;
    z-index: 4;
    width: 2px;
    margin-left: -1px;
    background: var(--on-surface);
    pointer-events: none;
  }

  .playhead::before {
    content: "";
    position: absolute;
    top: 0;
    left: 50%;
    transform: translateX(-50%);
    border: 5px solid transparent;
    border-top-color: var(--on-surface);
    border-bottom: none;
  }

  .times {
    display: flex;
    justify-content: space-between;
  }
</style>
