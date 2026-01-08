<script lang="ts">
  import { onMount } from 'svelte';
  import Sidebar from '$lib/components/sidebar.svelte';
  import { authApi } from '$lib/api';

  let oldPassword = '';
  let newPassword = '';
  let confirmPassword = '';
  let message = '';
  let error = '';
  let loading = false;
  let messageType: 'success' | 'error' = 'success';

  onMount(async () => {
    try {
      const response = await fetch('/api/auth/verify', {
        credentials: 'include'
      });
      
      if (!response.ok) {
        window.location.href = '/login';
      }
    } catch (err) {
      window.location.href = '/login';
    }
  });

  async function handleChangePassword(e: Event) {
    e.preventDefault();
    message = '';
    error = '';

    if (!oldPassword || !newPassword || !confirmPassword) {
      error = 'All fields are required';
      messageType = 'error';
      return;
    }

    if (newPassword.length < 4) {
      error = 'New password must be at least 4 characters long';
      messageType = 'error';
      return;
    }

    if (newPassword !== confirmPassword) {
      error = 'New passwords do not match';
      messageType = 'error';
      return;
    }

    loading = true;

    try {
      const response = await fetch('/api/auth/change-password', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        credentials: 'include',
        body: JSON.stringify({
          old_password: oldPassword,
          new_password: newPassword,
        }),
      });

      const data = await response.json();

      if (response.ok) {
        message = data.message || 'Password changed successfully';
        messageType = 'success';
        
        oldPassword = '';
        newPassword = '';
        confirmPassword = '';
        
        setTimeout(() => {
          localStorage.removeItem('harbory_token');
          window.location.href = '/login';
        }, 2000);
      } else {
        error = data.error || 'Failed to change password';
        messageType = 'error';
      }
    } catch (err) {
      error = 'Failed to connect to server';
      messageType = 'error';
    } finally {
      loading = false;
    }
  }

  async function handleLogout() {
    try {
      await authApi.logout();
      localStorage.removeItem('harbory_token');
      window.location.href = '/login';
    } catch (error) {
      console.error('Logout failed:', error);
      localStorage.removeItem('harbory_token');
      window.location.href = '/login';
    }
  }
</script>

<div class="flex">
  <Sidebar />
  
  <main class="flex-1 ml-64 p-8 bg-gray-50 min-h-screen">
    <div class="max-w-4xl mx-auto">
      <div class="mb-8">
        <h1 class="text-3xl font-bold text-gray-800">Profile Settings</h1>
        <p class="text-gray-600 mt-2">Manage your account settings and security</p>
      </div>

      <div class="grid gap-6">
        <div class="bg-white rounded-lg shadow-md p-6">
          <div class="flex items-center gap-3 mb-6">
            <svg class="w-6 h-6 text-[#1f7d53]" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z"></path>
            </svg>
            <h2 class="text-xl font-semibold text-gray-800">Change Password</h2>
          </div>

          <form on:submit={handleChangePassword} class="space-y-4">
            <div>
              <label for="oldPassword" class="block text-sm font-medium text-gray-700 mb-2">
                Current Password
              </label>
              <input
                id="oldPassword"
                type="password"
                bind:value={oldPassword}
                disabled={loading}
                placeholder="Enter your current password"
                class="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-[#1f7d53] focus:border-transparent transition-all outline-none disabled:bg-gray-100"
                required
              />
            </div>

            <div>
              <label for="newPassword" class="block text-sm font-medium text-gray-700 mb-2">
                New Password
              </label>
              <input
                id="newPassword"
                type="password"
                bind:value={newPassword}
                disabled={loading}
                placeholder="Enter your new password"
                class="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-[#1f7d53] focus:border-transparent transition-all outline-none disabled:bg-gray-100"
                required
              />
            </div>

            <div>
              <label for="confirmPassword" class="block text-sm font-medium text-gray-700 mb-2">
                Confirm New Password
              </label>
              <input
                id="confirmPassword"
                type="password"
                bind:value={confirmPassword}
                disabled={loading}
                placeholder="Confirm your new password"
                class="w-full px-4 py-3 border border-gray-300 rounded-lg focus:ring-2 focus:ring-[#1f7d53] focus:border-transparent transition-all outline-none disabled:bg-gray-100"
                required
              />
            </div>

            {#if message}
              <div class="bg-green-50 border border-green-200 text-green-700 px-4 py-3 rounded-lg text-sm flex items-center gap-2">
                <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                  <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd"></path>
                </svg>
                <span>{message}</span>
              </div>
            {/if}

            {#if error}
              <div class="bg-red-50 border border-red-200 text-red-700 px-4 py-3 rounded-lg text-sm flex items-center gap-2">
                <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                  <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z" clip-rule="evenodd"></path>
                </svg>
                <span>{error}</span>
              </div>
            {/if}

            <button
              type="submit"
              disabled={loading}
              class="w-full bg-[#1f7d53] hover:bg-[#186542] text-white font-semibold py-3 px-4 rounded-lg transition-colors disabled:bg-gray-300 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            >
              {#if loading}
                <div class="animate-spin rounded-full h-5 w-5 border-b-2 border-white"></div>
                <span>Changing Password...</span>
              {:else}
                <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
                </svg>
                <span>Change Password</span>
              {/if}
            </button>
          </form>
        </div>

        <div class="bg-white rounded-lg shadow-md p-6">
          <div class="flex items-center gap-3 mb-6">
            <svg class="w-6 h-6 text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.065 2.572c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.572 1.065c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.065-2.572c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z"></path>
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z"></path>
            </svg>
            <h2 class="text-xl font-semibold text-gray-800">Account Actions</h2>
          </div>

          <div class="space-y-3">
            <button
              on:click={handleLogout}
              class="w-full flex items-center justify-center gap-3 px-6 py-3 bg-red-50 hover:bg-red-600 text-red-600 hover:text-white border-2 border-red-200 hover:border-red-600 rounded-lg font-semibold transition-all"
            >
              <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1"></path>
              </svg>
              <span>Logout</span>
            </button>
          </div>
        </div>

        <div class="bg-white rounded-lg shadow-md p-6">
          <div class="flex items-center gap-3 mb-6">
            <svg class="w-6 h-6 text-gray-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
            </svg>
            <h2 class="text-xl font-semibold text-gray-800">Security Information</h2>
          </div>

          <div class="space-y-3 text-sm text-gray-600">
            <div class="flex items-start gap-2">
              <svg class="w-5 h-5 text-[#1f7d53] flex-shrink-0 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
                <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd"></path>
              </svg>
              <p>Sessions expire after 24 hours of inactivity</p>
            </div>
            <div class="flex items-start gap-2">
              <svg class="w-5 h-5 text-[#1f7d53] flex-shrink-0 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
                <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd"></path>
              </svg>
              <p>Changing your password will log you out from all devices</p>
            </div>
            <div class="flex items-start gap-2">
              <svg class="w-5 h-5 text-[#1f7d53] flex-shrink-0 mt-0.5" fill="currentColor" viewBox="0 0 20 20">
                <path fill-rule="evenodd" d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z" clip-rule="evenodd"></path>
              </svg>
              <p>Use a strong password with at least 4 characters</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  </main>
</div>
