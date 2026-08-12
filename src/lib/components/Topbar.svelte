<script lang="ts">
</script>

<nav class="topbar" aria-label="Navegación principal">
  <div class="brand">
    <img class="brand-logo" src="/logo.svg" alt="" aria-hidden="true" />
    <span class="brand-name">Vesper</span>
  </div>
  <ol class="topbar-nav">
    <li><a class="nav-link is-active" href="/home">Descargar</a></li>
    <li><button class="nav-link" type="button" disabled>Editar</button></li>
  </ol>
</nav>

<style>
  .topbar {
    position: sticky;
    top: 0;
    z-index: 50;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-4);
    height: 80px;
    padding: 0 var(--container-padding);
    background: rgba(19, 18, 27, 0.72);
    backdrop-filter: blur(14px);
    -webkit-backdrop-filter: blur(14px);
    border-bottom: 1px solid var(--glass-edge);
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 12px;
  }

  .brand-logo {
    width: 44px;
    height: 44px;
    border-radius: var(--radius-full);
    box-shadow: var(--glow-primary);
    object-fit: contain;
  }

  .brand-name {
    color: var(--on-surface);
    font-family: var(--font-display);
    font-size: 22px;
    font-weight: 700;
    line-height: 28px;
    letter-spacing: 0.05em;
    text-transform: uppercase;
  }

  .topbar-nav {
    display: flex;
    list-style: none;
    gap: var(--space-1);
  }

  .topbar-nav a,
  .topbar-nav button {
    display: inline-block;
    padding: 8px 14px;
    border: none;
    border-radius: var(--radius);
    background: transparent;
    color: var(--on-surface-variant);
    font-family: var(--font-display);
    font-size: 14px;
    font-weight: 500;
    line-height: 20px;
    text-decoration: none;
    cursor: pointer;
    transition:
      background-color var(--ease),
      color var(--ease);
  }

  .topbar-nav a:hover,
  .topbar-nav button:hover:not(:disabled) {
    background: var(--hover-fill);
    color: var(--on-surface);
  }

  .topbar-nav button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }

  .topbar-nav a.is-active {
    background: var(--active-fill);
    color: var(--primary);
  }
</style>
