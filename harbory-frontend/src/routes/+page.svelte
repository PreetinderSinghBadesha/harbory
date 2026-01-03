<script lang="ts">
  import { onMount } from 'svelte';
  import harboryLogo from '$lib/assets/harbory-logo.png';

  onMount(() => {
    // Redirect to dashboard after a short delay
    setTimeout(() => {
      window.location.href = '/dashboard';
    }, 2000);
  });
</script>

<div class="flex items-center justify-center min-h-screen">
  <div class="text-center space-y-6">
    <img src={harboryLogo} alt="Harbory Logo" class="w-32 h-32 mx-auto animate-pulse"/>
    <div>
      <h1 class="text-5xl font-bold text-gray-800 mb-2">Welcome to Harbory</h1>
      <p class="text-xl text-gray-600">Docker Management Made Simple</p>
    </div>
    <div class="flex items-center justify-center gap-2 text-gray-500">
      <div class="animate-spin rounded-full h-5 w-5 border-b-2 border-[#1f7d53]"></div>
      <span>Redirecting to dashboard...</span>
    </div>
  </div>
</div>
