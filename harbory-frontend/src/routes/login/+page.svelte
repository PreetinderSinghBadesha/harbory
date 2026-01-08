<script lang="ts">
  import { onMount } from 'svelte';
  import harboryLogo from '$lib/assets/harbory-logo.png';

  let password = '';
  let error = '';
  let loading = false;

  onMount(async () => {
    try {
      const response = await fetch('/api/auth/verify', {
        credentials: 'include'
      });
      
      if (response.ok) {
        window.location.href = '/dashboard';
      }
    } catch (err) {
      console.error('Error verifying authentication:', err);
    }
  });

  async function handleLogin(e: Event) {
    e.preventDefault();
    error = '';
    loading = true;

    try {
      const response = await fetch('/api/auth/login', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        credentials: 'include',
        body: JSON.stringify({ password }),
      });

      if (response.ok) {
        const data = await response.json();
        localStorage.setItem('harbory_token', data.token);
        window.location.href = '/dashboard';
      } else {
        const data = await response.json();
        error = data.error || 'Invalid password';
      }
    } catch (err) {
      error = 'Failed to connect to server';
    } finally {
      loading = false;
    }
  }
</script>

<div class="flex items-center justify-center min-h-screen bg-gradient-to-br from-gray-50 to-gray-100">
  <div class="w-full max-w-md">
    <div class="bg-white shadow-2xl rounded-2xl p-8 space-y-6">
      <div class="text-center space-y-4">
        <img src={harboryLogo} alt="Harbory Logo" class="w-20 h-20 mx-auto"/>
        <div>
          <h1 class="text-3xl font-bold text-gray-800">Welcome to Harbory</h1>
          <p class="text-gray-600 mt-2">Docker Management Made Simple</p>
        </div>
      </div>

      <!-- Login Form -->
      <form on:submit={handleLogin} class="space-y-4">
        <div>
          <label for="password" class="block text-sm font-medium text-gray-700 mb-2">
            Password
          </label>
          <input
            id="password"
            type="password"
            bind:value={password}
            disabled={loading}
            placeholder="Enter your password"
            class="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-[#1f7d53] focus:border-transparent transition-all outline-none disabled:bg-gray-100 disabled:cursor-not-allowed"
            required
          />
        </div>

        {#if error}
          <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm">
            {error}
          </div>
        {/if}

        <button
          type="submit"
          disabled={loading || !password}
          class="w-full bg-[#1f7d53] hover:bg-[#186542] text-white font-semibold py-3 px-4 rounded-lg transition-colors disabled:bg-gray-300 disabled:cursor-not-allowed flex items-center justify-center gap-2"
        >
          {#if loading}
            <div class="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></div>
            <span>Logging in...</span>
          {:else}
            <span>Login</span>
          {/if}
        </button>
      </form>

      <div class="text-center text-sm text-gray-500 pt-4 border-t">
        <p>Default password: <code class="bg-gray-100 px-2 py-1 rounded">admin</code></p>
      </div>
    </div>
  </div>
</div>

<style>
  code {
    font-family: 'Courier New', monospace;
  }
</style>
