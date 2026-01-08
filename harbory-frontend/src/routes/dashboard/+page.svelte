<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { containerApi, imageApi, volumeApi, networkApi, systemApi } from '$lib/api';

  let containers: any[] = $state([]);
  let images: any[] = $state([]);
  let volumes: any[] = $state([]);
  let networks: any[] = $state([]);
  let systemStats: any = $state(null);
  let loading = $state(true);
  let refreshInterval: number | null = null;

  async function fetchSystemStats() {
    try {
      const statsRes = await systemApi.getStats();
      if (statsRes.data) {
        systemStats = statsRes.data;
      }
    } catch (error) {
      console.error('Error fetching system stats:', error);
    }
  }

  onMount(async () => {
    try {
      const [containersRes, imagesRes, volumesRes, networksRes] = await Promise.all([
        containerApi.getAll(),
        imageApi.getAll(),
        volumeApi.getAll(),
        networkApi.getAll()
      ]);

      containers = containersRes.data || [];
      images = imagesRes.data || [];
      volumes = volumesRes.data?.Volumes || [];
      networks = networksRes.data || [];

      await fetchSystemStats();
      
      refreshInterval = window.setInterval(fetchSystemStats, 5000);
    } catch (error) {
      console.error('Error fetching data:', error);
    } finally {
      loading = false;
    }
  });

  onDestroy(() => {
    if (refreshInterval) {
      clearInterval(refreshInterval);
    }
  });

  function formatBytes(bytes: number, decimals = 2) {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(dm)) + ' ' + sizes[i];
  }
</script>

<div class="space-y-6">
  <div>
    <h1 class="text-3xl font-bold text-gray-800">Dashboard</h1>
    <p class="text-gray-600 mt-2">Overview of your Docker environment</p>
  </div>

  {#if loading}
    <div class="flex items-center justify-center py-12">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-[#1f7d53]"></div>
    </div>
  {:else}
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
      <!-- Containers Card -->
      <div class="bg-white rounded-lg shadow-md p-6 hover:shadow-lg transition-shadow">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-gray-500 text-sm font-medium">Containers</p>
            <p class="text-3xl font-bold text-gray-800 mt-2">{containers.length || 0}</p>
          </div>
          <div class="bg-blue-100 rounded-full p-3">
            <svg class="w-6 h-6 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"></path>
            </svg>
          </div>
        </div>
      </div>

      <!-- Images Card -->
      <div class="bg-white rounded-lg shadow-md p-6 hover:shadow-lg transition-shadow">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-gray-500 text-sm font-medium">Images</p>
            <p class="text-3xl font-bold text-gray-800 mt-2">{images.length || 0}</p>
          </div>
          <div class="bg-green-100 rounded-full p-3">
            <svg class="w-6 h-6 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 16l4.586-4.586a2 2 0 012.828 0L16 16m-2-2l1.586-1.586a2 2 0 012.828 0L20 14m-6-6h.01M6 20h12a2 2 0 002-2V6a2 2 0 00-2-2H6a2 2 0 00-2 2v12a2 2 0 002 2z"></path>
            </svg>
          </div>
        </div>
      </div>

      <!-- Volumes Card -->
      <div class="bg-white rounded-lg shadow-md p-6 hover:shadow-lg transition-shadow">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-gray-500 text-sm font-medium">Volumes</p>
            <p class="text-3xl font-bold text-gray-800 mt-2">{volumes.length || 0}</p>
          </div>
          <div class="bg-purple-100 rounded-full p-3">
            <svg class="w-6 h-6 text-purple-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 19a2 2 0 01-2-2V7a2 2 0 012-2h4l2 2h4a2 2 0 012 2v1M5 19h14a2 2 0 002-2v-5a2 2 0 00-2-2H9a2 2 0 00-2 2v5a2 2 0 01-2 2z"></path>
            </svg>
          </div>
        </div>
      </div>

      <!-- Networks Card -->
      <div class="bg-white rounded-lg shadow-md p-6 hover:shadow-lg transition-shadow">
        <div class="flex items-center justify-between">
          <div>
            <p class="text-gray-500 text-sm font-medium">Networks</p>
            <p class="text-3xl font-bold text-gray-800 mt-2">{networks.length || 0}</p>
          </div>
          <div class="bg-orange-100 rounded-full p-3">
            <svg class="w-6 h-6 text-orange-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 12a9 9 0 01-9 9m9-9a9 9 0 00-9-9m9 9H3m9 9a9 9 0 01-9-9m9 9c1.657 0 3-4.03 3-9s-1.343-9-3-9m0 18c-1.657 0-3-4.03-3-9s1.343-9 3-9m-9 9a9 9 0 019-9"></path>
            </svg>
          </div>
        </div>
      </div>
    </div>

    <!-- System Monitoring Section -->
    {#if systemStats}
    <div class="bg-white rounded-lg shadow-md p-6">
      <h2 class="text-xl font-bold text-gray-800 mb-4">System Monitoring</h2>
      
      <div class="grid grid-cols-1 md:grid-cols-3 gap-6">
        <!-- CPU Card -->
        <div class="border border-gray-200 rounded-lg p-4">
          <div class="flex items-center justify-between mb-3">
            <h3 class="text-sm font-semibold text-gray-700">CPU</h3>
            <div class="bg-indigo-100 rounded-full p-2">
              <svg class="w-4 h-4 text-indigo-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 3v2m6-2v2M9 19v2m6-2v2M5 9H3m2 6H3m18-6h-2m2 6h-2M7 19h10a2 2 0 002-2V7a2 2 0 00-2-2H7a2 2 0 00-2 2v10a2 2 0 002 2zM9 9h6v6H9V9z"></path>
              </svg>
            </div>
          </div>
          <div class="space-y-2">
            <div class="flex justify-between text-sm">
              <span class="text-gray-600">Cores:</span>
              <span class="font-semibold text-gray-800">{systemStats.cpu.cores}</span>
            </div>
            <div class="flex justify-between text-sm">
              <span class="text-gray-600">Goroutines:</span>
              <span class="font-semibold text-gray-800">{systemStats.cpu.goroutines}</span>
            </div>
          </div>
        </div>

        <!-- Memory Card -->
        <div class="border border-gray-200 rounded-lg p-4">
          <div class="flex items-center justify-between mb-3">
            <h3 class="text-sm font-semibold text-gray-700">Memory</h3>
            <div class="bg-pink-100 rounded-full p-2">
              <svg class="w-4 h-4 text-pink-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 7v10c0 2.21 3.582 4 8 4s8-1.79 8-4V7M4 7c0 2.21 3.582 4 8 4s8-1.79 8-4M4 7c0-2.21 3.582-4 8-4s8 1.79 8 4"></path>
              </svg>
            </div>
          </div>
          <div class="space-y-2">
            <div class="flex justify-between text-sm">
              <span class="text-gray-600">Total:</span>
              <span class="font-semibold text-gray-800">{systemStats.memory.total_mb} MB</span>
            </div>
            <div class="flex justify-between text-sm">
              <span class="text-gray-600">Used:</span>
              <span class="font-semibold text-gray-800">{systemStats.memory.used_mb} MB</span>
            </div>
            <div class="flex justify-between text-sm">
              <span class="text-gray-600">Free:</span>
              <span class="font-semibold text-gray-800">{systemStats.memory.free_mb} MB</span>
            </div>
            <div class="mt-3">
              <div class="flex justify-between text-xs text-gray-600 mb-1">
                <span>Usage</span>
                <span>{systemStats.memory.usage_percent.toFixed(1)}%</span>
              </div>
              <div class="w-full bg-gray-200 rounded-full h-2">
                <div 
                  class="h-2 rounded-full transition-all duration-500"
                  class:bg-green-500={systemStats.memory.usage_percent < 60}
                  class:bg-yellow-500={systemStats.memory.usage_percent >= 60 && systemStats.memory.usage_percent < 80}
                  class:bg-red-500={systemStats.memory.usage_percent >= 80}
                  style="width: {systemStats.memory.usage_percent}%"
                ></div>
              </div>
            </div>
          </div>
        </div>

        <!-- Disk Card -->
        <div class="border border-gray-200 rounded-lg p-4">
          <div class="flex items-center justify-between mb-3">
            <h3 class="text-sm font-semibold text-gray-700">Disk</h3>
            <div class="bg-cyan-100 rounded-full p-2">
              <svg class="w-4 h-4 text-cyan-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M8 7H5a2 2 0 00-2 2v9a2 2 0 002 2h14a2 2 0 002-2V9a2 2 0 00-2-2h-3m-1 4l-3 3m0 0l-3-3m3 3V4"></path>
              </svg>
            </div>
          </div>
          <div class="space-y-2">
            <div class="flex justify-between text-sm">
              <span class="text-gray-600">Total:</span>
              <span class="font-semibold text-gray-800">{systemStats.disk.total_gb} GB</span>
            </div>
            <div class="flex justify-between text-sm">
              <span class="text-gray-600">Used:</span>
              <span class="font-semibold text-gray-800">{systemStats.disk.used_gb} GB</span>
            </div>
            <div class="flex justify-between text-sm">
              <span class="text-gray-600">Free:</span>
              <span class="font-semibold text-gray-800">{systemStats.disk.free_gb} GB</span>
            </div>
            <div class="mt-3">
              <div class="flex justify-between text-xs text-gray-600 mb-1">
                <span>Usage</span>
                <span>{systemStats.disk.usage_percent.toFixed(1)}%</span>
              </div>
              <div class="w-full bg-gray-200 rounded-full h-2">
                <div 
                  class="h-2 rounded-full transition-all duration-500"
                  class:bg-green-500={systemStats.disk.usage_percent < 60}
                  class:bg-yellow-500={systemStats.disk.usage_percent >= 60 && systemStats.disk.usage_percent < 80}
                  class:bg-red-500={systemStats.disk.usage_percent >= 80}
                  style="width: {systemStats.disk.usage_percent}%"
                ></div>
              </div>
            </div>
          </div>
        </div>
      </div>

      <!-- Docker System Info -->
      <div class="mt-6 grid grid-cols-1 md:grid-cols-2 gap-4">
        <div class="border border-gray-200 rounded-lg p-4">
          <h3 class="text-sm font-semibold text-gray-700 mb-3">Docker Info</h3>
          <div class="space-y-2">
            <div class="flex justify-between text-sm">
              <span class="text-gray-600">Docker Version:</span>
              <span class="font-semibold text-gray-800">{systemStats.docker.version}</span>
            </div>
            <div class="flex justify-between text-sm">
              <span class="text-gray-600">API Version:</span>
              <span class="font-semibold text-gray-800">{systemStats.docker.server_version}</span>
            </div>
            <div class="flex justify-between text-sm">
              <span class="text-gray-600">Running Containers:</span>
              <span class="font-semibold text-gray-800">{systemStats.docker.containers_running} / {systemStats.docker.containers_total}</span>
            </div>
          </div>
        </div>

        <div class="border border-gray-200 rounded-lg p-4">
          <h3 class="text-sm font-semibold text-gray-700 mb-3">System Info</h3>
          <div class="space-y-2">
            <div class="flex justify-between text-sm">
              <span class="text-gray-600">OS:</span>
              <span class="font-semibold text-gray-800">{systemStats.system.os}</span>
            </div>
            <div class="flex justify-between text-sm">
              <span class="text-gray-600">Architecture:</span>
              <span class="font-semibold text-gray-800">{systemStats.system.architecture}</span>
            </div>
            <div class="flex justify-between text-sm">
              <span class="text-gray-600">Total Images:</span>
              <span class="font-semibold text-gray-800">{systemStats.docker.images}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
    {/if}

    <!-- Recent Containers -->
    <div class="bg-white rounded-lg shadow-md p-6">
      <h2 class="text-xl font-bold text-gray-800 mb-4">Recent Containers</h2>
      {#if containers?.data?.length > 0}
        <div class="overflow-x-auto">
          <table class="w-full">
            <thead class="bg-gray-50 border-b">
              <tr>
                <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Name</th>
                <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Image</th>
                <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Status</th>
                <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">State</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-gray-200">
              {#each containers.data.slice(0, 5) as container}
                <tr class="hover:bg-gray-50">
                  <td class="px-4 py-3 text-sm text-gray-800">{container.Names?.[0]?.replace('/', '') || 'N/A'}</td>
                  <td class="px-4 py-3 text-sm text-gray-600">{container.Image || 'N/A'}</td>
                  <td class="px-4 py-3 text-sm text-gray-600">{container.Status || 'N/A'}</td>
                  <td class="px-4 py-3">
                    <span class="px-2 py-1 text-xs rounded-full {container.State === 'running' ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}">
                      {container.State || 'unknown'}
                    </span>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {:else}
        <p class="text-gray-500 text-center py-8">No containers found</p>
      {/if}
    </div>
  {/if}
</div>
