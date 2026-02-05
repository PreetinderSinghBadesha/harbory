<script lang="ts">
  import { onDestroy, onMount } from 'svelte';

  let repoUrl = $state('');
  let hasDockerfile = $state(true);
  let dockerfilePath = $state('./Dockerfile');
  let framework = $state('');
  let deploying = $state(false);
  let logs: string[] = $state([]);
  let currentStep = $state('');
  let deploymentStatus = $state<'idle' | 'deploying' | 'success' | 'error'>('idle');
  let ws: WebSocket | null = null;

  // GitHub Browser state
  let githubToken = $state('');
  let showToken = $-state(false);
  let showBrowser = $state(false);
  let activeTab = $state<'my-repos' | 'search'>('my-repos');
  let searchQuery = $state('');
  let repositories: any[] = $state([]);
  let loadingRepos = $state(false);
  let repoError = $state('');

  const frameworks = ['node', 'react', 'go', 'flutter'];

  onMount(() => {
    // Load saved token from sessionStorage
    const savedToken = sessionStorage.getItem('github_token');
    if (savedToken) {
      githubToken = savedToken;
    }
  });

  function saveToken() {
    if (githubToken) {
      sessionStorage.setItem('github_token', githubToken);
    }
  }

  function clearToken() {
    githubToken = '';
    sessionStorage.removeItem('github_token');
    repositories = [];
  }

  async function fetchMyRepos() {
    if (!githubToken) {
      repoError = 'Please enter a GitHub token first';
      return;
    }

    loadingRepos = true;
    repoError = '';
    
    try {
      const response = await fetch('/api/github/user/repos', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ token: githubToken, page: 1 }),
        credentials: 'include'
      });

      if (!response.ok) {
        const error = await response.text();
        throw new Error(error || 'Failed to fetch repositories');
      }

      repositories = await response.json();
    } catch (error: any) {
      repoError = error.message || 'Failed to load repositories';
      repositories = [];
    } finally {
      loadingRepos = false;
    }
  }

  async function searchRepos() {
    if (!searchQuery.trim()) {
      repoError = 'Please enter a search query';
      return;
    }

    loadingRepos = true;
    repoError = '';
    
    try {
      const response = await fetch('/api/github/search', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ 
          query: searchQuery, 
          page: 1,
          token: githubToken || undefined 
        }),
        credentials: 'include'
      });

      if (!response.ok) {
        const error = await response.text();
        throw new Error(error || 'Failed to search repositories');
      }

      const data = await response.json();
      repositories = data.items || [];
    } catch (error: any) {
      repoError = error.message || 'Failed to search repositories';
      repositories = [];
    } finally {
      loadingRepos = false;
    }
  }

  function selectRepository(repo: any) {
    repoUrl = repo.clone_url;
    showBrowser = false;
    window.scrollTo({ top: document.body.scrollHeight, behavior: 'smooth' });
  }

  $effect(() => {
    if (showBrowser && activeTab === 'my-repos' && githubToken && repositories.length === 0) {
      fetchMyRepos();
    }
  });

  function handleDeploy() {
    if (!repoUrl) {
      alert('Please enter a repository URL');
      return;
    }

    if (!hasDockerfile && !framework) {
      alert('Please select a framework or provide a Dockerfile');
      return;
    }

    deploying = true;
    deploymentStatus = 'deploying';
    logs = [];
    currentStep = 'Connecting...';

    ws = new WebSocket('ws://localhost:8080/api/deploy/ws');

    ws.onopen = () => {
      currentStep = 'Connected - Sending deployment request...';

      const payload = {
        repo_url: repoUrl,
        has_dockerfile: hasDockerfile,
        dockerfile_path: hasDockerfile ? dockerfilePath : '',
        framework: !hasDockerfile ? framework : '',
        github_token: githubToken || undefined
      };
      
      ws?.send(JSON.stringify(payload));
    };

    ws.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data);
        
        switch (message.type) {
          case 'status':
            currentStep = message.message;
            logs = [...logs, `[STATUS] ${message.message}`];
            break;
          case 'log':
            logs = [...logs, message.message];
            break;
          case 'error':
            deploymentStatus = 'error';
            currentStep = 'Deployment failed';
            logs = [...logs, `[ERROR] ${message.message}`];
            deploying = false;
            break;
          case 'success':
            deploymentStatus = 'success';
            currentStep = 'Deployment completed successfully!';
            logs = [...logs, `[SUCCESS] ${message.message}`];
            deploying = false;
            break;
        }
        
        setTimeout(() => {
          const logContainer = document.getElementById('log-container');
          if (logContainer) {
            logContainer.scrollTop = logContainer.scrollHeight;
          }
        }, 10);
      } catch (error) {
        console.error('Error parsing message:', error);
      }
    };

    ws.onerror = (error) => {
      console.error('WebSocket error:', error);
      deploymentStatus = 'error';
      currentStep = 'Connection error';
      logs = [...logs, '[ERROR] WebSocket connection failed'];
      deploying = false;
    };

    ws.onclose = () => {
      if (deploying) {
        deploying = false;
        if (deploymentStatus === 'deploying') {
          deploymentStatus = 'error';
          currentStep = 'Connection closed unexpectedly';
        }
      }
    };
  }

  function resetForm() {
    repoUrl = '';
    hasDockerfile = true;
    dockerfilePath = './Dockerfile';
    framework = '';
    logs = [];
    currentStep = '';
    deploymentStatus = 'idle';
    deploying = false;
  }

  onDestroy(() => {
    if (ws) {
      ws.close();
    }
  });
</script>

<div class="space-y-6">
  <div>
    <h1 class="text-3xl font-bold text-gray-800">Deploy Application</h1>
    <p class="text-gray-600 mt-2">Deploy your application from a Git repository</p>
  </div>

  <!-- GitHub Repository Browser -->
  <div class="bg-white rounded-lg shadow-md p-6">
    <button
      onclick={() => showBrowser = !showBrowser}
      class="w-full flex items-center justify-between text-lg font-bold text-gray-800 hover:text-[#1f7d53] transition-colors"
    >
      <span>🔍 Browse GitHub Repositories</span>
      <svg 
        class="w-5 h-5 transform transition-transform {showBrowser ? 'rotate-180' : ''}" 
        fill="none" 
        stroke="currentColor" 
        viewBox="0 0 24 24"
      >
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"></path>
      </svg>
    </button>

    {#if showBrowser}
      <div class="mt-4 space-y-4">
        <!-- Token Input Section -->
        <div class="border-b pb-4">
          <label class="block text-sm font-medium text-gray-700 mb-2">
            GitHub Personal Access Token (Optional)
          </label>
          <p class="text-xs text-gray-500 mb-2">
            Provide a token to access private repositories. 
            <a 
              href="https://github.com/settings/tokens/new" 
              target="_blank" 
              class="text-[#1f7d53] hover:underline"
            >
              Create one here →
            </a>
          </p>
          <div class="flex gap-2">
            <div class="flex-1 relative">
              <input
                type={showToken ? 'text' : 'password'}
                bind:value={githubToken}
                onchange={saveToken}
                placeholder="ghp_xxxxxxxxxxxxxxxxxxxx"
                class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-[#1f7d53] focus:border-transparent"
              />
              <button
                onclick={() => showToken = !showToken}
                class="absolute right-2 top-1/2 -translate-y-1/2 text-gray-500 hover:text-gray-700"
              >
                {showToken ? '🙈' : '👁️'}
              </button>
            </div>
            {#if githubToken}
              <button
                onclick={clearToken}
                class="px-4 py-2 bg-gray-200 text-gray-700 rounded-lg hover:bg-gray-300"
              >
                Clear
              </button>
            {/if}
          </div>
        </div>

        <!-- Tabs -->
        <div class="flex gap-4 border-b">
          <button
            onclick={() => { activeTab = 'my-repos'; repositories = []; fetchMyRepos(); }}
            class="pb-2 px-4 font-medium transition-colors {activeTab === 'my-repos' ? 'text-[#1f7d53] border-b-2 border-[#1f7d53]' : 'text-gray-500 hover:text-gray-700'}"
            disabled={!githubToken}
          >
            📁 My Repositories
          </button>
          <button
            onclick={() => { activeTab = 'search'; repositories = []; }}
            class="pb-2 px-4 font-medium transition-colors {activeTab === 'search' ? 'text-[#1f7d53] border-b-2 border-[#1f7d53]' : 'text-gray-500 hover:text-gray-700'}"
          >
            🔍 Search GitHub
          </button>
        </div>

        <!-- Search Tab Content -->
        {#if activeTab === 'search'}
          <div class="space-y-3">
            <div class="flex gap-2">
              <input
                type="text"
                bind:value={searchQuery}
                onkeypress={(e) => e.key === 'Enter' && searchRepos()}
                placeholder="Search repositories (e.g., react, nodejs, docker)..."
                class="flex-1 px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-[#1f7d53] focus:border-transparent"
              />
              <button
                onclick={searchRepos}
                disabled={loadingRepos}
                class="px-6 py-2 bg-[#1f7d53] text-white rounded-lg hover:bg-[#186642] disabled:bg-gray-300 transition-colors"
              >
                {loadingRepos ? 'Searching...' : 'Search'}
              </button>
            </div>
          </div>
        {/if}

        <!-- Repository List -->
        {#if loadingRepos}
          <div class="text-center py-8">
            <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-[#1f7d53] mx-auto"></div>
            <p class="mt-4 text-gray-600">Loading repositories...</p>
          </div>
        {:else if repoError}
          <div class="bg-red-50 border border-red-200 rounded-lg p-4 text-red-700">
            ⚠️ {repoError}
          </div>
        {:else if repositories.length > 0}
          <div class="space-y-3 max-h-96 overflow-y-auto">
            {#each repositories as repo}
              <div class="border border-gray-200 rounded-lg p-4 hover:border-[#1f7d53] hover:shadow-md transition-all">
                <div class="flex items-start justify-between gap-4">
                  <div class="flex-1 min-w-0">
                    <div class="flex items-center gap-2 mb-1">
                      <h3 class="font-semibold text-gray-800 truncate">{repo.full_name}</h3>
                      {#if repo.private}
                        <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-yellow-100 text-yellow-800">
                          🔒 Private
                        </span>
                      {:else}
                        <span class="inline-flex items-center px-2 py-0.5 rounded text-xs font-medium bg-green-100 text-green-800">
                          🌐 Public
                        </span>
                      {/if}
                    </div>
                    <p class="text-sm text-gray-600 line-clamp-2">{repo.description || 'No description'}</p>
                    <div class="flex items-center gap-4 mt-2 text-xs text-gray-500">
                      {#if repo.language}
                        <span>💻 {repo.language}</span>
                      {/if}
                      <span>⭐ {repo.stargazers_count}</span>
                      <span>🔀 {repo.forks_count}</span>
                    </div>
                  </div>
                  <button
                    onclick={() => selectRepository(repo)}
                    class="px-4 py-2 bg-[#1f7d53] text-white rounded-lg hover:bg-[#186642] transition-colors whitespace-nowrap"
                  >
                    Select
                  </button>
                </div>
              </div>
            {/each}
          </div>
        {:else if activeTab === 'my-repos' && !githubToken}
          <div class="text-center py-8 text-gray-500">
            Please enter a GitHub token to view your repositories
          </div>
        {:else if activeTab === 'search' && !searchQuery}
          <div class="text-center py-8 text-gray-500">
            Enter a search query above to find repositories
          </div>
        {:else}
          <div class="text-center py-8 text-gray-500">
            No repositories found
          </div>
        {/if}
      </div>
    {/if}
  </div>

  <div class="grid grid-cols-1 lg:grid-cols-2 gap-6">
    <div class="bg-white rounded-lg shadow-md p-6">
      <h2 class="text-xl font-bold text-gray-800 mb-4">Configuration</h2>
      
      <div class="space-y-4">
        <div>
          <label class="block text-sm font-medium text-gray-700 mb-2">
            Repository URL *
          </label>
          <input
            type="text"
            bind:value={repoUrl}
            placeholder="https://github.com/username/repo"
            disabled={deploying}
            class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-[#1f7d53] focus:border-transparent disabled:bg-gray-100"
          />
        </div>

        <div class="flex items-center gap-3">
          <input
            type="checkbox"
            id="hasDockerfile"
            bind:checked={hasDockerfile}
            disabled={deploying}
            class="w-4 h-4 text-[#1f7d53] border-gray-300 rounded focus:ring-[#1f7d53]"
          />
          <label for="hasDockerfile" class="text-sm font-medium text-gray-700">
            Repository has a Dockerfile
          </label>
        </div>

        {#if hasDockerfile}
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-2">
              Dockerfile Path
            </label>
            <input
              type="text"
              bind:value={dockerfilePath}
              placeholder="./Dockerfile"
              disabled={deploying}
              class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-[#1f7d53] focus:border-transparent disabled:bg-gray-100"
            />
          </div>
        {:else}
          <div>
            <label class="block text-sm font-medium text-gray-700 mb-2">
              Framework *
            </label>
            <select
              bind:value={framework}
              disabled={deploying}
              class="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-[#1f7d53] focus:border-transparent disabled:bg-gray-100"
            >
              <option value="">Select a framework</option>
              {#each frameworks as fw}
                <option value={fw}>{fw.charAt(0).toUpperCase() + fw.slice(1)}</option>
              {/each}
            </select>
          </div>
        {/if}

        <div class="flex gap-3 pt-4">
          <button
            onclick={handleDeploy}
            disabled={deploying}
            class="flex-1 px-6 py-3 bg-[#1f7d53] text-white rounded-lg hover:bg-[#186642] disabled:bg-gray-300 disabled:cursor-not-allowed transition-colors font-semibold"
          >
            {deploying ? 'Deploying...' : 'Deploy Application'}
          </button>
          
          {#if deploymentStatus !== 'idle' && !deploying}
            <button
              onclick={resetForm}
              class="px-6 py-3 bg-gray-200 text-gray-700 rounded-lg hover:bg-gray-300 transition-colors font-semibold"
            >
              Reset
            </button>
          {/if}
        </div>
      </div>
    </div>

    <div class="bg-white rounded-lg shadow-md p-6">
      <h2 class="text-xl font-bold text-gray-800 mb-4">Deployment Status</h2>

      <div class="mb-4">
        {#if deploymentStatus === 'idle'}
          <div class="px-4 py-2 bg-gray-100 text-gray-600 rounded-lg text-center">
            Ready to deploy
          </div>
        {:else if deploymentStatus === 'deploying'}
          <div class="px-4 py-2 bg-blue-100 text-blue-800 rounded-lg text-center flex items-center justify-center gap-2">
            <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-blue-800"></div>
            Deploying...
          </div>
        {:else if deploymentStatus === 'success'}
          <div class="px-4 py-2 bg-green-100 text-green-800 rounded-lg text-center flex items-center justify-center gap-2">
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7"></path>
            </svg>
            Deployment Successful
          </div>
        {:else if deploymentStatus === 'error'}
          <div class="px-4 py-2 bg-red-100 text-red-800 rounded-lg text-center flex items-center justify-center gap-2">
            <svg class="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12"></path>
            </svg>
            Deployment Failed
          </div>
        {/if}
      </div>

      {#if currentStep}
        <div class="mb-4 p-3 bg-gray-50 rounded-lg">
          <p class="text-sm font-medium text-gray-700">{currentStep}</p>
        </div>
      {/if}

      <div class="mt-4">
        <h3 class="text-sm font-semibold text-gray-700 mb-2">Deployment Logs</h3>
        <div
          id="log-container"
          class="bg-gray-900 text-green-400 rounded-lg p-4 h-96 overflow-y-auto font-mono text-xs"
        >
          {#if logs.length === 0}
            <p class="text-gray-500">No logs yet...</p>
          {:else}
            {#each logs as log}
              <div class="mb-1">{log}</div>
            {/each}
          {/if}
        </div>
      </div>
    </div>
  </div>
</div>
