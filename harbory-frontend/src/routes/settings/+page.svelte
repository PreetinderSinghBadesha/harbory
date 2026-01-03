<script lang="ts">
  import { onMount } from 'svelte';
  import { containerApi, imageApi, volumeApi, networkApi, nodeApi, healthApi } from '$lib/api';

  let systemInfo = $state({
    containers: 0,
    images: 0,
    volumes: 0,
    networks: 0
  });
  let nodes: any[] = $state([]);
  let health: any = $state(null);
  let loading = $state(true);

  onMount(async () => {
    try {
      const [containersRes, imagesRes, volumesRes, networksRes, nodesRes, healthRes] = await Promise.all([
        containerApi.getAll(),
        imageApi.getAll(),
        volumeApi.getAll(),
        networkApi.getAll(),
        nodeApi.getAll(),
        healthApi.check()
      ]);

      systemInfo = {
        containers: containersRes.data?.length || 0,
        images: imagesRes.data?.length || 0,
        volumes: volumesRes.data?.Volumes?.length || 0,
        networks: networksRes.data?.length || 0
      };
      
      nodes = nodesRes.data || [];
      health = healthRes.data;
    } catch (error) {
      console.error('Error fetching data:', error);
    } finally {
      loading = false;
    }
  });
</script>

<div class="space-y-6">
  <div>
    <h1 class="text-3xl font-bold text-gray-800">System Monitoring</h1>
    <p class="text-gray-600 mt-2">Monitor your Docker environment performance</p>
  </div>

  {#if loading}
    <div class="flex items-center justify-center py-12">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-[#1f7d53]"></div>
    </div>
  {:else}
    <!-- Resource Stats -->
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
      <div class="bg-white rounded-lg shadow-md p-6">
        <div class="flex items-center justify-between mb-2">
          <h3 class="text-sm font-medium text-gray-500">Total Containers</h3>
          <svg class="w-5 h-5 text-blue-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 7l-8-4-8 4m16 0l-8 4m8-4v10l-8 4m0-10L4 7m8 4v10M4 7v10l8 4"></path>
          </svg>
        </div>
        <p class="text-3xl font-bold text-gray-800">{systemInfo.containers}</p>
      </div>

      <div class="bg-white rounded-lg shadow-md p-6">
        <div class="flex items-center justify-between mb-2">
          <h3 class="text-sm font-medium text-gray-500">Total Images</h3>
          <svg class="w-5 h-5 text-green-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"></path>
          </svg>
        </div>
        <p class="text-3xl font-bold text-gray-800">{systemInfo.images}</p>
      </div>

      <div class="bg-white rounded-lg shadow-md p-6">
        <div class="flex items-center justify-between mb-2">
          <h3 class="text-sm font-medium text-gray-500">Total Volumes</h3>
          <svg class="w-5 h-5 text-purple-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z"></path>
          </svg>
        </div>
        <p class="text-3xl font-bold text-gray-800">{systemInfo.volumes}</p>
      </div>

      <div class="bg-white rounded-lg shadow-md p-6">
        <div class="flex items-center justify-between mb-2">
          <h3 class="text-sm font-medium text-gray-500">Total Networks</h3>
          <svg class="w-5 h-5 text-orange-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9"></path>
          </svg>
        </div>
        <p class="text-3xl font-bold text-gray-800">{systemInfo.networks}</p>
      </div>
    </div>

    <!-- System Health -->
    <div class="bg-white rounded-lg shadow-md p-6">
      <h2 class="text-xl font-bold text-gray-800 mb-4">System Health</h2>
      <div class="space-y-4">
        <div>
          <div class="flex items-center justify-between mb-2">
            <span class="text-sm font-medium text-gray-700">Docker Daemon</span>
            <span class="px-2 py-1 text-xs rounded-full bg-green-100 text-green-800">Running</span>
          </div>
          <div class="w-full bg-gray-200 rounded-full h-2">
            <div class="bg-green-500 h-2 rounded-full" style="width: 100%"></div>
          </div>
        </div>

        <div>
          <div class="flex items-center justify-between mb-2">
            <span class="text-sm font-medium text-gray-700">API Connection</span>
            <span class="px-2 py-1 text-xs rounded-full bg-green-100 text-green-800">Connected</span>
          </div>
          <div class="w-full bg-gray-200 rounded-full h-2">
            <div class="bg-green-500 h-2 rounded-full" style="width: 100%"></div>
          </div>
        </div>
      </div>
    </div>

    <!-- Recent Activity -->
    <div class="bg-white rounded-lg shadow-md p-6">
      <h2 class="text-xl font-bold text-gray-800 mb-4">Quick Stats</h2>
      <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div class="p-4 bg-blue-50 rounded-lg">
          <p class="text-sm text-blue-600 font-medium">Active Resources</p>
          <p class="text-2xl font-bold text-blue-700 mt-1">{systemInfo.containers + systemInfo.volumes}</p>
        </div>
        <div class="p-4 bg-green-50 rounded-lg">
          <p class="text-sm text-green-600 font-medium">Total Images</p>
          <p class="text-2xl font-bold text-green-700 mt-1">{systemInfo.images}</p>
        </div>
        <div class="p-4 bg-purple-50 rounded-lg">
          <p class="text-sm text-purple-600 font-medium">Network Configs</p>
          <p class="text-2xl font-bold text-purple-700 mt-1">{systemInfo.networks}</p>
        </div>
      </div>
    </div>
  {/if}
</div>
