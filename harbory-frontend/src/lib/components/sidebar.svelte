<script lang="ts">
  import dashboardIcon from '$lib/assets/dashboard.png';
  import analyticsIcon from '$lib/assets/analytics.png';
  import dockerIcon from '$lib/assets/docker.png';
  import projectsIcon from '$lib/assets/projects.png';
  import harboryLogo from '$lib/assets/harbory-logo.png';
  import { authApi } from '$lib/api';
  
  let { class: className = '' } = $props();
  
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

<aside class="w-64 h-screen bg-[#18230f] text-slate-100 flex flex-col fixed left-0 top-0 overflow-y-auto z-40 {className}">
<div class="p-6 border-b border-white/10">
    <div class="flex items-center gap-3">
        <img src={harboryLogo} alt="Harbory Logo" class="w-10 h-10 object-contain"/>
        <div class="flex flex-col gap-1">
            <h2 class="text-2xl font-bold m-0">Harbory</h2>
            <p class="text-sm text-slate-400">Admin Console</p>
        </div>
    </div>
</div>
  
<nav class="flex flex-col py-4 gap-1">
    <a href="/dashboard" class="flex items-center gap-3 px-6 py-3 mx-2 rounded-lg text-slate-300 no-underline transition-all hover:bg-[#1f7d53]">
        <img src={dashboardIcon} alt="Dashboard Logo" class="w-5 h-5"/>
        <span class="font-semibold">Dashboard</span>
    </a>
    <a href="/deploy" class="flex items-center gap-3 px-6 py-3 mx-2 rounded-lg text-slate-300 no-underline transition-all hover:bg-[#1f7d53] hover:text-slate-100">
        <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M7 16a4 4 0 01-.88-7.903A5 5 0 1115.9 6L16 6a5 5 0 011 9.9M15 13l-3-3m0 0l-3 3m3-3v12"></path>
        </svg>
        <span class="font-semibold">Deploy</span>
    </a>
    <a href="/docker" class="flex items-center gap-3 px-6 py-3 mx-2 rounded-lg text-slate-300 no-underline transition-all hover:bg-[#1f7d53] hover:text-slate-100">
        <img src={dockerIcon} alt="Docker Logo" class="w-5 h-5"/>
        <span class="font-semibold">Docker</span>
    </a>
</nav>

<div class="mt-auto">
    <div class="border-t border-white/10 p-4">
        <a href="/profile" class="flex items-center gap-3 px-6 py-3 rounded-lg text-slate-300 no-underline transition-all hover:bg-[#1f7d53] hover:text-slate-100">
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M16 7a4 4 0 11-8 0 4 4 0 018 0zM12 14a7 7 0 00-7 7h14a7 7 0 00-7-7z"></path>
            </svg>
            <span class="font-semibold">Profile</span>
        </a>
    </div>
    <!-- <div class="border-t border-white/10 p-4">
        <button 
            on:click={handleLogout}
            class="flex items-center gap-3 px-6 py-3 w-full rounded-lg text-slate-300 bg-transparent border-none cursor-pointer transition-all hover:bg-red-600 hover:text-white"
        >
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17 16l4-4m0 0l-4-4m4 4H7m6 4v1a3 3 0 01-3 3H6a3 3 0 01-3-3V7a3 3 0 013-3h4a3 3 0 013 3v1"></path>
            </svg>
            <span class="font-semibold">Logout</span>
        </button>
    </div> -->
</div>
</aside>
