<script lang="ts">
  import { onMount } from 'svelte';
  import { containerApi, imageApi, volumeApi, networkApi } from '$lib/api';

  let activeTab = $state('containers');
  let containers: any[] = $state([]);
  let images: any[] = $state([]);
  let volumes: any[] = $state([]);
  let networks: any[] = $state([]);
  let loading = $state(true);
  let selectedItem: any = $state(null);
  let showModal = $state(false);
  let modalType = $state('');

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

  function showInfo(item: any, type: string) {
    selectedItem = item;
    modalType = type;
    showModal = true;
  }

  function closeModal() {
    showModal = false;
    selectedItem = null;
    modalType = '';
  }

  function formatBytes(bytes: number): string {
    if (bytes === 0) return '0 Bytes';
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
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
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Actions</th>
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
                          {container.Ports.map((p: any) => p.PublicPort ? `${p.PublicPort}:${p.PrivatePort}` : p.PrivatePort).join(', ')}
                        {:else}
                          -
                        {/if}
                      </td>
                      <td class="px-4 py-3">
                        <button
                          onclick={() => showInfo(container, 'container')}
                          class="text-[#1f7d53] hover:text-[#186642] transition-colors"
                          title="View Details"
                        >
                          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                          </svg>
                        </button>
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
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Actions</th>
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
                      <td class="px-4 py-3 text-sm text-gray-600">{formatBytes(image.Size)}</td>
                      <td class="px-4 py-3 text-sm text-gray-600">{new Date(image.Created * 1000).toLocaleDateString()}</td>
                      <td class="px-4 py-3">
                        <button
                          onclick={() => showInfo(image, 'image')}
                          class="text-[#1f7d53] hover:text-[#186642] transition-colors"
                          title="View Details"
                        >
                          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                          </svg>
                        </button>
                      </td>
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
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Actions</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-gray-200">
                  {#each volumes as volume}
                    <tr class="hover:bg-gray-50">
                      <td class="px-4 py-3 text-sm text-gray-800 font-medium">{volume.Name}</td>
                      <td class="px-4 py-3 text-sm text-gray-600">{volume.Driver}</td>
                      <td class="px-4 py-3 text-sm text-gray-600 truncate max-w-md">{volume.Mountpoint}</td>
                      <td class="px-4 py-3 text-sm text-gray-600">{volume.Scope}</td>
                      <td class="px-4 py-3">
                        <button
                          onclick={() => showInfo(volume, 'volume')}
                          class="text-[#1f7d53] hover:text-[#186642] transition-colors"
                          title="View Details"
                        >
                          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                          </svg>
                        </button>
                      </td>
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
                    <th class="px-4 py-3 text-left text-xs font-medium text-gray-500 uppercase">Actions</th>
                  </tr>
                </thead>
                <tbody class="divide-y divide-gray-200">
                  {#each networks as network}
                    <tr class="hover:bg-gray-50">
                      <td class="px-4 py-3 text-sm text-gray-800 font-medium">{network.Name}</td>
                      <td class="px-4 py-3 text-sm text-gray-600 font-mono text-xs">{network.Id?.substring(0, 12)}</td>
                      <td class="px-4 py-3 text-sm text-gray-600">{network.Driver}</td>
                      <td class="px-4 py-3 text-sm text-gray-600">{network.Scope}</td>
                      <td class="px-4 py-3">
                        <button
                          onclick={() => showInfo(network, 'network')}
                          class="text-[#1f7d53] hover:text-[#186642] transition-colors"
                          title="View Details"
                        >
                          <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"></path>
                          </svg>
                        </button>
                      </td>
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

<!-- Info Modal -->
{#if showModal && selectedItem}
  <div class="fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center p-4" onclick={closeModal}>
    <div class="bg-white rounded-lg shadow-xl max-w-3xl w-full max-h-[90vh] overflow-hidden" onclick={(e) => e.stopPropagation()}>
      <div class="flex items-center justify-between p-6 border-b border-gray-200">
        <h2 class="text-2xl font-bold text-gray-800">
          {modalType === 'container' ? 'Container Details' : 
           modalType === 'image' ? 'Image Details' : 
           modalType === 'volume' ? 'Volume Details' : 
           'Network Details'}
        </h2>
        <button onclick={closeModal} class="text-gray-400 hover:text-gray-600 transition-colors" aria-label="Close modal">
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
          </svg>
        </button>
      </div>

      <div class="p-6 overflow-y-auto max-h-[calc(90vh-140px)]">
        {#if modalType === 'container'}
          <div class="space-y-4">
            <div class="grid grid-cols-2 gap-4">
              <div>
                <p class="text-sm font-medium text-gray-500">Name</p>
                <p class="text-sm text-gray-800 mt-1">{selectedItem.Names?.[0]?.replace('/', '') || 'N/A'}</p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">Container ID</p>
                <p class="text-sm text-gray-800 font-mono mt-1">{selectedItem.Id?.substring(0, 12)}</p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">Image</p>
                <p class="text-sm text-gray-800 mt-1">{selectedItem.Image}</p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">Image ID</p>
                <p class="text-sm text-gray-800 font-mono mt-1">{selectedItem.ImageID?.substring(7, 19)}</p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">State</p>
                <p class="text-sm mt-1">
                  <span class="px-2 py-1 text-xs rounded-full {selectedItem.State === 'running' ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}">
                    {selectedItem.State}
                  </span>
                </p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">Status</p>
                <p class="text-sm text-gray-800 mt-1">{selectedItem.Status}</p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">Created</p>
                <p class="text-sm text-gray-800 mt-1">{new Date(selectedItem.Created * 1000).toLocaleString()}</p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">Command</p>
                <p class="text-sm text-gray-800 font-mono mt-1">{selectedItem.Command}</p>
              </div>
            </div>
            {#if selectedItem.Ports?.length > 0}
              <div>
                <p class="text-sm font-medium text-gray-500 mb-2">Ports</p>
                <div class="bg-gray-50 rounded-lg p-3 space-y-1">
                  {#each selectedItem.Ports as port}
                    <p class="text-sm text-gray-800">
                      {port.IP || '0.0.0.0'}:{port.PublicPort || '-'} → {port.PrivatePort}/{port.Type}
                    </p>
                  {/each}
                </div>
              </div>
            {/if}
            {#if selectedItem.Mounts?.length > 0}
              <div>
                <p class="text-sm font-medium text-gray-500 mb-2">Mounts</p>
                <div class="bg-gray-50 rounded-lg p-3 space-y-1">
                  {#each selectedItem.Mounts as mount}
                    <p class="text-sm text-gray-800">
                      {mount.Source} → {mount.Destination} ({mount.Mode})
                    </p>
                  {/each}
                </div>
              </div>
            {/if}
            {#if selectedItem.NetworkSettings?.Networks}
              <div>
                <p class="text-sm font-medium text-gray-500 mb-2">Networks</p>
                <div class="bg-gray-50 rounded-lg p-3 space-y-2">
                  {#each Object.entries(selectedItem.NetworkSettings.Networks) as [name, network]}
                    <div>
                      <p class="text-sm font-semibold text-gray-800">{name}</p>
                      <p class="text-sm text-gray-600">IP: {(network as any).IPAddress || 'N/A'}</p>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        {:else if modalType === 'image'}
          <div class="space-y-4">
            <div class="grid grid-cols-2 gap-4">
              <div>
                <p class="text-sm font-medium text-gray-500">Repository</p>
                <p class="text-sm text-gray-800 mt-1">{selectedItem.RepoTags?.[0]?.split(':')[0] || '<none>'}</p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">Tag</p>
                <p class="text-sm text-gray-800 mt-1">{selectedItem.RepoTags?.[0]?.split(':')[1] || '<none>'}</p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">Image ID</p>
                <p class="text-sm text-gray-800 font-mono mt-1">{selectedItem.Id?.substring(7, 19)}</p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">Size</p>
                <p class="text-sm text-gray-800 mt-1">{formatBytes(selectedItem.Size)}</p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">Created</p>
                <p class="text-sm text-gray-800 mt-1">{new Date(selectedItem.Created * 1000).toLocaleString()}</p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">Virtual Size</p>
                <p class="text-sm text-gray-800 mt-1">{formatBytes(selectedItem.VirtualSize)}</p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">Containers</p>
                <p class="text-sm text-gray-800 mt-1">{selectedItem.Containers || 0}</p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">Parent ID</p>
                <p class="text-sm text-gray-800 font-mono mt-1">{selectedItem.ParentId?.substring(7, 19) || 'N/A'}</p>
              </div>
            </div>
            {#if selectedItem.RepoTags?.length > 1}
              <div>
                <p class="text-sm font-medium text-gray-500 mb-2">All Tags</p>
                <div class="bg-gray-50 rounded-lg p-3 space-y-1">
                  {#each selectedItem.RepoTags as tag}
                    <p class="text-sm text-gray-800">{tag}</p>
                  {/each}
                </div>
              </div>
            {/if}
            {#if selectedItem.RepoDigests?.length > 0}
              <div>
                <p class="text-sm font-medium text-gray-500 mb-2">Repo Digests</p>
                <div class="bg-gray-50 rounded-lg p-3 space-y-1">
                  {#each selectedItem.RepoDigests as digest}
                    <p class="text-sm text-gray-800 font-mono break-all">{digest}</p>
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        {:else if modalType === 'volume'}
          <div class="space-y-4">
            <div class="grid grid-cols-2 gap-4">
              <div>
                <p class="text-sm font-medium text-gray-500">Name</p>
                <p class="text-sm text-gray-800 mt-1">{selectedItem.Name}</p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">Driver</p>
                <p class="text-sm text-gray-800 mt-1">{selectedItem.Driver}</p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">Scope</p>
                <p class="text-sm text-gray-800 mt-1">{selectedItem.Scope}</p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">Created</p>
                <p class="text-sm text-gray-800 mt-1">{selectedItem.CreatedAt ? new Date(selectedItem.CreatedAt).toLocaleString() : 'N/A'}</p>
              </div>
            </div>
            <div>
              <p class="text-sm font-medium text-gray-500 mb-2">Mountpoint</p>
              <div class="bg-gray-50 rounded-lg p-3">
                <p class="text-sm text-gray-800 font-mono break-all">{selectedItem.Mountpoint}</p>
              </div>
            </div>
            {#if selectedItem.Labels && Object.keys(selectedItem.Labels).length > 0}
              <div>
                <p class="text-sm font-medium text-gray-500 mb-2">Labels</p>
                <div class="bg-gray-50 rounded-lg p-3 space-y-1">
                  {#each Object.entries(selectedItem.Labels) as [key, value]}
                    <p class="text-sm text-gray-800"><span class="font-semibold">{key}:</span> {value}</p>
                  {/each}
                </div>
              </div>
            {/if}
            {#if selectedItem.Options && Object.keys(selectedItem.Options).length > 0}
              <div>
                <p class="text-sm font-medium text-gray-500 mb-2">Options</p>
                <div class="bg-gray-50 rounded-lg p-3 space-y-1">
                  {#each Object.entries(selectedItem.Options) as [key, value]}
                    <p class="text-sm text-gray-800"><span class="font-semibold">{key}:</span> {value}</p>
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        {:else if modalType === 'network'}
          <div class="space-y-4">
            <div class="grid grid-cols-2 gap-4">
              <div>
                <p class="text-sm font-medium text-gray-500">Name</p>
                <p class="text-sm text-gray-800 mt-1">{selectedItem.Name}</p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">Network ID</p>
                <p class="text-sm text-gray-800 font-mono mt-1">{selectedItem.Id?.substring(0, 12)}</p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">Driver</p>
                <p class="text-sm text-gray-800 mt-1">{selectedItem.Driver}</p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">Scope</p>
                <p class="text-sm text-gray-800 mt-1">{selectedItem.Scope}</p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">Internal</p>
                <p class="text-sm text-gray-800 mt-1">{selectedItem.Internal ? 'Yes' : 'No'}</p>
              </div>
              <div>
                <p class="text-sm font-medium text-gray-500">Created</p>
                <p class="text-sm text-gray-800 mt-1">{selectedItem.Created ? new Date(selectedItem.Created).toLocaleString() : 'N/A'}</p>
              </div>
            </div>
            {#if selectedItem.IPAM?.Config?.length > 0}
              <div>
                <p class="text-sm font-medium text-gray-500 mb-2">IPAM Config</p>
                <div class="bg-gray-50 rounded-lg p-3 space-y-2">
                  {#each selectedItem.IPAM.Config as config}
                    <div>
                      <p class="text-sm text-gray-800"><span class="font-semibold">Subnet:</span> {config.Subnet || 'N/A'}</p>
                      <p class="text-sm text-gray-800"><span class="font-semibold">Gateway:</span> {config.Gateway || 'N/A'}</p>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
            {#if selectedItem.Containers && Object.keys(selectedItem.Containers).length > 0}
              <div>
                <p class="text-sm font-medium text-gray-500 mb-2">Connected Containers</p>
                <div class="bg-gray-50 rounded-lg p-3 space-y-2">
                  {#each Object.entries(selectedItem.Containers) as [id, container]}
                    <div>
                      <p class="text-sm font-semibold text-gray-800">{(container as any).Name}</p>
                      <p class="text-sm text-gray-600">IP: {(container as any).IPv4Address || 'N/A'}</p>
                      <p class="text-sm text-gray-600 font-mono">MAC: {(container as any).MacAddress || 'N/A'}</p>
                    </div>
                  {/each}
                </div>
              </div>
            {/if}
            {#if selectedItem.Options && Object.keys(selectedItem.Options).length > 0}
              <div>
                <p class="text-sm font-medium text-gray-500 mb-2">Options</p>
                <div class="bg-gray-50 rounded-lg p-3 space-y-1">
                  {#each Object.entries(selectedItem.Options) as [key, value]}
                    <p class="text-sm text-gray-800"><span class="font-semibold">{key}:</span> {value}</p>
                  {/each}
                </div>
              </div>
            {/if}
            {#if selectedItem.Labels && Object.keys(selectedItem.Labels).length > 0}
              <div>
                <p class="text-sm font-medium text-gray-500 mb-2">Labels</p>
                <div class="bg-gray-50 rounded-lg p-3 space-y-1">
                  {#each Object.entries(selectedItem.Labels) as [key, value]}
                    <p class="text-sm text-gray-800"><span class="font-semibold">{key}:</span> {value}</p>
                  {/each}
                </div>
              </div>
            {/if}
          </div>
        {/if}
      </div>

      <div class="flex justify-end p-6 border-t border-gray-200">
        <button
          onclick={closeModal}
          class="px-4 py-2 bg-gray-200 text-gray-800 rounded-lg hover:bg-gray-300 transition-colors"
        >
          Close
        </button>
      </div>
    </div>
  </div>
{/if}
