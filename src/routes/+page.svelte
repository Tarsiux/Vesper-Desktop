<script lang="ts">
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";

  onMount(() => {
    const timer = setTimeout(() => goto("/home"), 2000);
    return () => clearTimeout(timer);
  });
</script>

<div class="loader">
  <div class="bar">
    <div class="fill"></div>
  </div>
  <p>Cargando...</p>
</div>

<style>
  .loader {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    width: 100%;
    height: 100vh;
    padding: 0 2rem;
    box-sizing: border-box;
  }

  .bar {
    width: min(80vw, 320px);
    height: 6px;
    border-radius: 3px;
    overflow: hidden;
    background: rgba(128, 128, 128, 0.3);
  }

  .fill {
    width: 100%;
    height: 100%;
    border-radius: 3px;
    background: #646cff;
    animation: load 2s ease-in-out forwards;
    transform-origin: left;
  }

  p {
    margin: 0;
    font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
    font-size: 1rem;
    color: #888;
  }

  @keyframes load {
    from {
      transform: scaleX(0);
    }
    to {
      transform: scaleX(1);
    }
  }
</style>