const API_BASE_URL = 'http://localhost:8080/api';

interface ApiResponse<T> {
  status: string;
  data?: T;
  error?: string;
}

async function fetchApi<T>(endpoint: string): Promise<ApiResponse<T>> {
  try {
    const response = await fetch(`${API_BASE_URL}${endpoint}`);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }
    const data = await response.json();
    
    if (data && typeof data === 'object' && 'status' in data) {
      return data;
    }

    return {
      status: 'ok',
      data: data
    };
  } catch (error) {
    console.error(`Error fetching ${endpoint}:`, error);
    return {
      status: 'error',
      error: error instanceof Error ? error.message : 'Unknown error'
    };
  }
}

export const containerApi = {
  getAll: () => fetchApi<any[]>('/containers'),
  getById: (id: string) => fetchApi<any>(`/containers/${id}`)
};

export const imageApi = {
  getAll: () => fetchApi<any[]>('/images'),
  getById: (id: string) => fetchApi<any>(`/images/${id}`)
};

export const volumeApi = {
  getAll: () => fetchApi<any>('/volumes'),
  getById: (name: string) => fetchApi<any>(`/volumes/${name}`)
};

export const networkApi = {
  getAll: () => fetchApi<any[]>('/networks'),
  getById: (id: string) => fetchApi<any>(`/networks/${id}`)
};

export const nodeApi = {
  getAll: () => fetchApi<any[]>('/nodes'),
  getById: (id: string) => fetchApi<any>(`/nodes/${id}`)
};

export const healthApi = {
  check: () => fetchApi<any>('/health')
};
