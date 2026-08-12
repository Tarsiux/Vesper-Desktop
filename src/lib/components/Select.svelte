<script lang="ts">
  export let options: string[] = [];
  export let value = "";
  export let placeholder = "Seleccionar";
  export let disabled = false;

  let open = false;
  let highlighted = 0;
  let openUp = false;
  let rootEl: HTMLDivElement;
  let triggerEl: HTMLButtonElement;

  function onWindowClick(e: MouseEvent) {
    if (rootEl && !rootEl.contains(e.target as Node)) {
      open = false;
    }
  }

  function onWindowKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      open = false;
    }
  }

  function openMenu() {
    const rect = triggerEl.getBoundingClientRect();
    const estHeight = Math.min(220, options.length * 34 + 12);
    openUp = rect.bottom + 6 + estHeight > window.innerHeight - 12;
    open = true;
    highlighted = Math.max(0, options.indexOf(value));
  }

  function select(option: string) {
    value = option;
    open = false;
    triggerEl?.focus();
  }

  function onTriggerClick() {
    if (disabled) return;
    if (open) {
      open = false;
    } else {
      openMenu();
    }
  }

  function onTriggerKeydown(e: KeyboardEvent) {
    if (disabled) return;
    if (e.key === "Escape") {
      open = false;
    } else if (open) {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        highlighted = (highlighted + 1) % options.length;
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        highlighted = (highlighted - 1 + options.length) % options.length;
      } else if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        value = options[highlighted] ?? options[0];
        open = false;
      }
    } else if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      openMenu();
    }
  }
</script>

<svelte:window on:click={onWindowClick} on:keydown={onWindowKeydown} />

<div class="select" bind:this={rootEl}>
  <button
    type="button"
    class="select-trigger"
    class:open
    class:disabled
    bind:this={triggerEl}
    aria-haspopup="listbox"
    aria-expanded={open}
    disabled={disabled}
    on:click={onTriggerClick}
    on:keydown={onTriggerKeydown}
  >
    <span class="select-value" class:placeholder={!value}>{value || placeholder}</span>
    <svg
      class="select-chevron"
      class:rotated={open}
      width="10"
      height="6"
      viewBox="0 0 10 6"
      fill="none"
      aria-hidden="true"
    >
      <path
        d="M1 1l4 4 4-4"
        stroke="currentColor"
        stroke-width="1.5"
        stroke-linecap="round"
        stroke-linejoin="round"
      />
    </svg>
  </button>

  {#if open}
    <ul class="select-menu" class:up={openUp} role="listbox">
      {#each options as option (option)}
        <li>
          <button
            type="button"
            role="option"
            tabindex="-1"
            aria-selected={option === value}
            class:selected={option === value}
            class:highlighted={option === options[highlighted]}
            on:click={() => select(option)}
            on:mouseenter={() => (highlighted = options.indexOf(option))}
          >
            {option}
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .select {
    position: relative;
  }

  .select-trigger {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    width: 100%;
    padding: 10px 2px;
    border: none;
    border-bottom: 2px solid var(--outline-variant);
    border-radius: 0;
    background: transparent;
    color: var(--on-surface);
    font-family: var(--font-mono);
    font-size: 14px;
    line-height: 20px;
    cursor: pointer;
    transition:
      border-color var(--ease),
      background-color var(--ease),
      box-shadow var(--ease);
  }

  .select-trigger:hover:not(:disabled) {
    border-bottom-color: var(--outline);
  }

  .select-trigger:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .select-trigger.open {
    border-bottom-color: var(--primary);
    background: rgba(157, 92, 255, 0.05);
    box-shadow: 0 3px 10px rgba(157, 92, 255, 0.18);
  }

  .select-value {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .select-value.placeholder {
    color: var(--text-secondary);
  }

  .select-chevron {
    flex-shrink: 0;
    color: var(--outline);
    transition:
      transform var(--ease),
      color var(--ease);
  }

  .select-chevron.rotated {
    transform: rotate(180deg);
  }

  .select-trigger.open .select-chevron {
    color: var(--primary);
  }

  .select-menu {
    position: absolute;
    top: calc(100% + 6px);
    left: 0;
    right: 0;
    z-index: 40;
    margin: 0;
    padding: var(--space-1);
    list-style: none;
    max-height: 220px;
    overflow-y: auto;
    background: var(--raised-surface);
    border: 1px solid var(--glass-edge);
    border-radius: var(--radius);
    box-shadow: var(--shadow-raised), var(--glow-primary);
    animation: fade-in 150ms ease;
  }

  .select-menu.up {
    top: auto;
    bottom: calc(100% + 6px);
  }

  .select-menu li {
    margin: 0;
    padding: 0;
  }

  .select-menu li button {
    display: block;
    width: 100%;
    padding: 7px 10px;
    border: none;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--on-surface-variant);
    font-family: var(--font-mono);
    font-size: 13px;
    line-height: 18px;
    text-align: left;
    cursor: pointer;
    transition:
      background-color var(--ease),
      color var(--ease);
  }

  .select-menu li button:hover,
  .select-menu li button.highlighted {
    background: var(--hover-fill);
    color: var(--on-surface);
  }

  .select-menu li button.selected,
  .select-menu li button.selected:hover,
  .select-menu li button.selected.highlighted {
    background: var(--active-fill);
    color: var(--primary);
  }
</style>
