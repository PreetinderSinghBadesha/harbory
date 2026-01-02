<script lang="ts">
  import favicon from '$lib/assets/favicon.svg';
  import Sidebar from '$lib/components/sidebar.svelte';
  import "../app.css";

  let { children } = $props();
</script>

<svelte:head>
  <link rel="icon" href={favicon} />
</svelte:head>

<div class="flex min-h-screen">
  <Sidebar />
  <main class="ml-64 p-8 bg-[#f5f8f4] flex-1 w-full">
    {@render children()}
  </main>
</div>
