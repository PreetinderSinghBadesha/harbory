<script lang="ts">
  import { onMount } from 'svelte';
  import { containerApi, imageApi, volumeApi, networkApi } from '$lib/api';

  let activeTab = $state('containers');
  let containers: any[] = $state([]);
  let images: any[] = $state([]);
  let volumes: any[] = $state([]);
  let networks: any[] = $state([]);
  let loading = $state(true);

  onMount(async () => {
    await fetchAllData();
  });

  async function fetchAllData() {
    try {
      loading = true;
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
    } catch (error) {
      console.error('Error fetching data:', error);
    } finally {
      loading = false;
    }
  }

  function setTab(tab: string) {
    activeTab = tab;
  }
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-3xl font-bold text-gray-800">Docker Management</h1>
      <p class="text-gray-600 mt-2">Manage all your Docker resources</p>
    </div>
    <button 
      onclick={fetchAllData}
      class="px-4 py-2 bg-[#1f7d53] text-white rounded-lg hover:bg-[#186642] transition-colors flex items-center gap-2"
    >
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
      </svg>
      Refresh
    </button>
  </div>

  <!-- Tabs -->
  <div class="bg-white rounded-lg shadow-md">
    <div class="border-b border-gray-200">
      <nav class="flex -mb-px">
        <button
          onclick={() => setTab('containers')}
          class="px-6 py-4 text-sm font-medium border-b-2 transition-colors {activeTab === 'containers' ? 'border-[#1f7d53] text-[#1f7d53]' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
        >
          Containers ({containers.length})
        </button>
        <button
          onclick={() => setTab('images')}
          class="px-6 py-4 text-sm font-medium border-b-2 transition-colors {activeTab === 'images' ? 'border-[#1f7d53] text-[#1f7d53]' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
        >
          Images ({images.length})
        </button>
        <button
          onclick={() => setTab('volumes')}
          class="px-6 py-4 text-sm font-medium border-b-2 transition-colors {activeTab === 'volumes' ? 'border-[#1f7d53] text-[#1f7d53]' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
        >
          Volumes ({volumes.length})
        </button>
        <button
          onclick={() => setTab('networks')}
          class="px-6 py-4 text-sm font-medium border-b-2 transition-colors {activeTab === 'networks' ? 'border-[#1f7d53] text-[#1f7d53]' : 'border-transparent text-gray-500 hover:text-gray-700 hover:border-gray-300'}"
        >
          Networks ({networks.length})
        </button>
      </nav>
    </div>

    <div class="p-6">
      {#if loading}
        <div class="flex items-center justify-center py-12">
          <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-[#1f7d53]"></div>
        </div>
      {:else}
        <!-- Containers Tab -->
        {#if activeTab === 'containers'}
          {#if containers.length === 0}
            <p class="text-gray-500 text-center py-8">No containers found</p>
          {:else}
            <div class="overflow-x-auto">
              <table class="w-full">
                <thead class="bg-gray-50 border-b">
                  <tr>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Name</th>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Image</th>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Status</th>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">State</th>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Ports</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-gray-200">
                  {#each containers as container}
                    <tr class="hover:bg-gray-50">
                      <td class="px-4 py-3 text-sm text-gray-800 font-medium">{container.Names?.[0]?.replace('/', '') || 'N/A'}</td>
                      <td class="px-4 py-3 text-sm text-gray-600">{container.Image}</td>
                      <td class="px-4 py-3 text-sm text-gray-600">{container.Status}</td>
                      <td class="px-4 py-3">
                        <span class="px-2 py-1 text-xs rounded-full {container.State === 'running' ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}">
                          {container.State}
                        </span>
                      </td>
                      <td class="px-4 py-3 text-sm text-gray-600">
                        {#if container.Ports?.length > 0}
                          {container.Ports.map(p => p.PublicPort ? `${p.PublicPort}:${p.PrivatePort}` : p.PrivatePort).join(', ')}
                        {:else}
                          -
                        {/if}
                      </td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {/if}
        {/if}

        <!-- Images Tab -->
        {#if activeTab === 'images'}
          {#if images.length === 0}
            <p class="text-gray-500 text-center py-8">No images found</p>
          {:else}
            <div class="overflow-x-auto">
              <table class="w-full">
                <thead class="bg-gray-50 border-b">
                  <tr>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Repository</th>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Tag</th>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Image ID</th>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Size</th>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Created</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-gray-200">
                  {#each images as image}
                    <tr class="hover:bg-gray-50">
                      <td class="px-4 py-3 text-sm text-gray-800 font-medium">
                        {image.RepoTags?.[0]?.split(':')[0] || '<none>'}
                      </td>
                      <td class="px-4 py-3 text-sm text-gray-600">
                        {image.RepoTags?.[0]?.split(':')[1] || '<none>'}
                      </td>
                      <td class="px-4 py-3 text-sm text-gray-600 font-mono text-xs">{image.Id?.substring(7, 19)}</td>
                      <td class="px-4 py-3 text-sm text-gray-600">{(image.Size / 1024 / 1024).toFixed(2)} MB</td>
                      <td class="px-4 py-3 text-sm text-gray-600">{new Date(image.Created * 1000).toLocaleDateString()}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {/if}
        {/if}

        <!-- Volumes Tab -->
        {#if activeTab === 'volumes'}
          {#if volumes.length === 0}
            <p class="text-gray-500 text-center py-8">No volumes found</p>
          {:else}
            <div class="overflow-x-auto">
              <table class="w-full">
                <thead class="bg-gray-50 border-b">
                  <tr>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Name</th>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Driver</th>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Mountpoint</th>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Scope</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-gray-200">
                  {#each volumes as volume}
                    <tr class="hover:bg-gray-50">
                      <td class="px-4 py-3 text-sm text-gray-800 font-medium">{volume.Name}</td>
                      <td class="px-4 py-3 text-sm text-gray-600">{volume.Driver}</td>
                      <td class="px-4 py-3 text-sm text-gray-600 truncate max-w-md">{volume.Mountpoint}</td>
                      <td class="px-4 py-3 text-sm text-gray-600">{volume.Scope}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {/if}
        {/if}

        <!-- Networks Tab -->
        {#if activeTab === 'networks'}
          {#if networks.length === 0}
            <p class="text-gray-500 text-center py-8">No networks found</p>
          {:else}
            <div class="overflow-x-auto">
              <table class="w-full">
                <thead class="bg-gray-50 border-b">
                  <tr>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Name</th>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">ID</th>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Driver</th>
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Scope</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-gray-200">
                  {#each networks as network}
                    <tr class="hover:bg-gray-50">
                      <td class="px-4 py-3 text-sm text-gray-800 font-medium">{network.Name}</td>
                      <td class="px-4 py-3 text-sm text-gray-600 font-mono text-xs">{network.Id?.substring(0, 12)}</td>
                      <td class="px-4 py-3 text-sm text-gray-600">{network.Driver}</td>
                      <td class="px-4 py-3 text-sm text-gray-600">{network.Scope}</td>
                    </tr>
                  {/each}
                </tbody>
              </table>
            </div>
          {/if}
        {/if}
      {/if}
    </div>
  </div>
</div>
