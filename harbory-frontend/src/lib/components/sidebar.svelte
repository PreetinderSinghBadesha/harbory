<script lang="ts">
  import dashboardIcon from '$lib/assets/dashboard.png';
  import analyticsIcon from '$lib/assets/analytics.png';
  import dockerIcon from '$lib/assets/docker.png';
  import projectsIcon from '$lib/assets/projects.png';
  import harboryLogo from '$lib/assets/harbory-logo.png';
  let { class: className = '' } = $props();
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
    <a href="/projects" class="flex items-center gap-3 px-6 py-3 mx-2 rounded-lg text-slate-300 no-underline transition-all hover:bg-[#1f7d53] hover:text-slate-100">
        <img src={projectsIcon} alt="Projects Logo" class="w-5 h-5"/>
        <span class="font-semibold">Containers</span>
    </a>
    <a href="/docker" class="flex items-center gap-3 px-6 py-3 mx-2 rounded-lg text-slate-300 no-underline transition-all hover:bg-[#1f7d53] hover:text-slate-100">
        <img src={dockerIcon} alt="Docker Logo" class="w-5 h-5"/>
        <span class="font-semibold">Docker</span>
    </a>
    <a href="/settings" class="flex items-center gap-3 px-6 py-3 mx-2 rounded-lg text-slate-300 no-underline transition-all hover:bg-[#1f7d53] hover:text-slate-100">
            <img src={analyticsIcon} alt="Monitoring Logo" class="w-5 h-5"/>
        <span class="font-semibold">Monitoring</span>
    </a>
</nav>
</aside>
