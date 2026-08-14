<script lang="ts">
  import { fly } from "svelte/transition";

  interface Props {
    /** Mensaje de error a mostrar. Vacío = oculto. */
    message?: string;
    /** Se llama cuando el usuario cierra el aviso (o expira el tiempo). */
    onDismiss?: () => void;
  }

  let { message = "", onDismiss }: Props = $props();

  let visible = $state(false);
  let hideTimer: ReturnType<typeof setTimeout> | undefined;
  let clearTimer: ReturnType<typeof setTimeout> | undefined;

  // Cada vez que llega un mensaje nuevo, se muestra el aviso y se programa el
  // cierre automático; si el mensaje se vacía desde fuera, se oculta.
  $effect(() => {
    if (message) {
      visible = true;
      clearTimeout(hideTimer);
      clearTimeout(clearTimer);
      hideTimer = setTimeout(dismiss, 8000);
    } else {
      visible = false;
    }
    return () => {
      clearTimeout(hideTimer);
      clearTimeout(clearTimer);
    };
  });

  function dismiss() {
    visible = false;
    clearTimeout(hideTimer);
    // Espera a que termine la transición de salida antes de vaciar el mensaje.
    clearTimer = setTimeout(() => onDismiss?.(), 220);
  }
</script>

{#if visible}
  <div
    class="toast-wrap"
    role="alert"
    aria-live="assertive"
    in:fly={{ y: -14, duration: 200 }}
    out:fly={{ y: -14, duration: 180 }}
  >
    <div class="toast">
      <span class="dot" aria-hidden="true"></span>
      <p class="body-md toast-msg">{message}</p>
      <button class="toast-close" type="button" onclick={dismiss} aria-label="Cerrar">
        &times;
      </button>
    </div>
  </div>
{/if}

<style>
  .toast-wrap {
    position: fixed;
    top: var(--space-4, 16px);
    left: 50%;
    transform: translateX(-50%);
    z-index: 200;
    width: min(92vw, 600px);
  }

  .toast {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 12px 16px;
    border: 1px solid rgba(255, 180, 171, 0.25);
    border-radius: var(--radius);
    background: rgba(147, 0, 10, 0.16);
    color: var(--on-error-container);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
    backdrop-filter: blur(8px);
  }

  .dot {
    flex-shrink: 0;
    width: 8px;
    height: 8px;
    margin-top: 8px;
    border-radius: var(--radius-full);
    background: var(--error);
    box-shadow: 0 0 8px rgba(255, 180, 171, 0.7);
  }

  .toast-msg {
    flex: 1;
    min-width: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .toast-close {
    flex-shrink: 0;
    margin: -4px -6px 0 0;
    padding: 4px 6px;
    border: none;
    background: none;
    color: inherit;
    font-size: 20px;
    line-height: 1;
    cursor: pointer;
    opacity: 0.7;
  }

  .toast-close:hover {
    opacity: 1;
  }
</style>
