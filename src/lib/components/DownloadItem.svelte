<script lang="ts">
  import ProgressRing from "./ProgressRing.svelte";
  import type { DownloadStatus } from "$lib/types";

  interface Props {
    /** Name of the file being downloaded. */
    fileName: string;
    /** Overall progress, from 0 to 100. */
    progress: number;
    /** Current phase of the download. */
    status: DownloadStatus;
    /** Live phase message (e.g. "Descargando video…"). */
    message?: string | null;
    /** Error text, if any. */
    error?: string | null;
    /** Action triggered when the remove button is pressed. */
    onRemove?: () => void;
  }

  let { fileName, progress, status, message, error, onRemove }: Props = $props();

  const isActive = $derived(
    status === "downloading" || status === "converting" || status === "merging"
  );
  const isDone = $derived(status === "completed");
  const isFailed = $derived(status === "error");

  const statusLabel = $derived.by(() => {
    switch (status) {
      case "downloading":
        return message ?? "Descargando…";
      case "converting":
        return message ?? "Convirtiendo…";
      case "merging":
        return message ?? "Uniendo…";
      case "completed":
        return "Completado";
      case "error":
        return "Error";
    }
  });
</script>

<div
  class="item"
  class:done={isDone}
  class:failed={isFailed}
  role="status"
  aria-live="polite"
>
  <div class="ring-wrap">
    <ProgressRing
      progress={progress}
      processing={isActive}
      label={statusLabel}
      ariaLabel={`Progreso de ${fileName}`}
    />
  </div>

  <div class="info">
    <p class="title body-md text-truncate" title={fileName}>{fileName}</p>

    {#if isFailed && error}
      <p class="state state-error label-sm">{error}</p>
    {:else if isDone}
      <p class="state state-done label-sm">
        <span class="check" aria-hidden="true">✓</span>
        100% descargado
      </p>
    {:else}
      <p class="state label-sm">{statusLabel}</p>
    {/if}
  </div>

  {#if onRemove}
    <button
      class="remove"
      type="button"
      onclick={onRemove}
      aria-label="Quitar de la cola"
    >
      &times;
    </button>
  {/if}
</div>

<style>
  .item {
    position: relative;
    display: flex;
    align-items: center;
    gap: var(--space-4);
    padding: var(--space-4);
    background: var(--raised-surface);
    border: 1px solid var(--glass-edge);
    border-radius: var(--radius-lg);
    box-shadow: var(--shadow-card);
  }

  .item.done {
    border-color: rgba(126, 217, 141, 0.35);
    box-shadow: var(--shadow-card), 0 0 16px rgba(126, 217, 141, 0.08);
  }

  .item.failed {
    border-color: rgba(255, 180, 171, 0.4);
  }

  .ring-wrap {
    flex-shrink: 0;
  }

  .info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .title {
    color: var(--on-surface);
  }

  .state {
    color: var(--on-surface-variant);
  }

  .state-done {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: #7dd88f;
  }

  .state-error {
    color: var(--error);
  }

  .check {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border-radius: var(--radius-full);
    background: rgba(126, 217, 141, 0.18);
    color: #7dd88f;
    font-size: 12px;
    line-height: 1;
  }

  .remove {
    position: absolute;
    top: var(--space-2);
    right: var(--space-2);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border: none;
    border-radius: var(--radius-full);
    background: transparent;
    color: var(--on-surface-variant);
    font-size: 20px;
    line-height: 1;
    cursor: pointer;
    transition:
      background var(--ease),
      color var(--ease);
  }

  .remove:hover {
    background: var(--hover-fill);
    color: var(--on-surface);
  }

  @media (max-width: 640px) {
    .item {
      gap: var(--space-3);
      padding: var(--space-3);
    }
  }
</style>
