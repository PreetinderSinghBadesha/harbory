<script lang="ts">
  import { onMount } from 'svelte';
  import { containerApi } from '$lib/api';

  let containers: any[] = $state([]);
  let loading = $state(true);
  let selectedContainer: any = $state(null);

  onMount(async () => {
    await fetchContainers();
  });

  async function fetchContainers() {
    try {
      loading = true;
      const response = await containerApi.getAll();
      containers = response.data || [];
    } catch (error) {
      console.error('Error fetching containers:', error);
    } finally {
      loading = false;
    }
  }

  function viewDetails(container: any) {
    selectedContainer = container;
  }

  function closeDetails() {
    selectedContainer = null;
  }
</script>

<div class="space-y-6">
  <div class="flex items-center justify-between">
    <div>
      <h1 class="text-3xl font-bold text-gray-800">Docker Containers</h1>
      <p class="text-gray-600 mt-2">Manage your Docker containers</p>
    </div>
    <button 
      onclick={fetchContainers}
      class="px-4 py-2 bg-[#1f7d53] text-white rounded-lg hover:bg-[#186642] transition-colors flex items-center gap-2"
    >
      <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15"></path>
      </svg>
      Refresh
    </button>
  </div>

  {#if loading}
    <div class="flex items-center justify-center py-12">
      <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-[#1f7d53]"></div>
    </div>
  {:else if containers.length === 0}
    <div class="bg-white rounded-lg shadow-md p-12 text-center">
      <svg class="w-16 h-16 text-gray-400 mx-auto mb-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4"></path>
      </svg>
      <h3 class="text-xl font-semibold text-gray-700 mb-2">No Containers Found</h3>
      <p class="text-gray-500">There are no Docker containers running on your system.</p>
    </div>
  {:else}
    <div class="grid grid-cols-1 lg:grid-cols-2 xl:grid-cols-3 gap-6">
      {#each containers as container}
        <div class="bg-white rounded-lg shadow-md p-6 hover:shadow-lg transition-shadow">
          <div class="flex items-start justify-between mb-4">
            <h3 class="text-lg font-semibold text-gray-800 truncate flex-1">
              {container.Names?.[0]?.replace('/', '') || 'Unnamed'}
            </h3>
            <span class="px-2 py-1 text-xs rounded-full {container.State === 'running' ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}">
              {container.State || 'unknown'}
            </span>
          </div>

          <div class="space-y-2 mb-4">
            <div class="flex items-center gap-2 text-sm">
              <span class="text-gray-500 font-medium">Image:</span>
              <span class="text-gray-700 truncate">{container.Image}</span>
            </div>
            <div class="flex items-center gap-2 text-sm">
              <span class="text-gray-500 font-medium">ID:</span>
              <span class="text-gray-700 font-mono text-xs">{container.Id?.substring(0, 12)}</span>
            </div>
            <div class="flex items-center gap-2 text-sm">
              <span class="text-gray-500 font-medium">Status:</span>
              <span class="text-gray-700">{container.Status}</span>
            </div>
          </div>

          <button 
            onclick={() => viewDetails(container)}
            class="w-full px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors"
          >
            View Details
          </button>
        </div>
      {/each}
    </div>
  {/if}
</div>

<!-- Details Modal -->
{#if selectedContainer}
  <div class="fixed inset-0 bg-black bg-opacity-50 flex items-center justify-center p-4 z-50" onclick={closeDetails}>
    <div class="bg-white rounded-lg shadow-xl max-w-2xl w-full max-h-[90vh] overflow-auto" onclick={(e) => e.stopPropagation()}>
      <div class="sticky top-0 bg-white border-b px-6 py-4 flex items-center justify-between">
        <h2 class="text-2xl font-bold text-gray-800">Container Details</h2>
        <button onclick={closeDetails} class="text-gray-500 hover:text-gray-700">
          <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
          </svg>
        </button>
      </div>
      <div class="p-6 space-y-4">
        <div>
          <h3 class="text-sm font-medium text-gray-500 mb-1">Container Name</h3>
          <p class="text-gray-800">{selectedContainer.Names?.[0]?.replace('/', '') || 'N/A'}</p>
        </div>
        <div>
          <h3 class="text-sm font-medium text-gray-500 mb-1">Container ID</h3>
          <p class="text-gray-800 font-mono text-sm">{selectedContainer.Id}</p>
        </div>
        <div>
          <h3 class="text-sm font-medium text-gray-500 mb-1">Image</h3>
          <p class="text-gray-800">{selectedContainer.Image}</p>
        </div>
        <div>
          <h3 class="text-sm font-medium text-gray-500 mb-1">State</h3>
          <span class="px-2 py-1 text-xs rounded-full {selectedContainer.State === 'running' ? 'bg-green-100 text-green-800' : 'bg-red-100 text-red-800'}">
            {selectedContainer.State}
          </span>
        </div>
        <div>
          <h3 class="text-sm font-medium text-gray-500 mb-1">Status</h3>
          <p class="text-gray-800">{selectedContainer.Status}</p>
        </div>
        {#if selectedContainer.Ports?.length > 0}
          <div>
            <h3 class="text-sm font-medium text-gray-500 mb-1">Ports</h3>
            <div class="space-y-1">
              {#each selectedContainer.Ports as port}
                <p class="text-gray-800 text-sm">
                  {port.PublicPort ? `${port.PublicPort} → ${port.PrivatePort}/${port.Type}` : `${port.PrivatePort}/${port.Type}`}
                </p>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    </div>
  </div>
{/if}
