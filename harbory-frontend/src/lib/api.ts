const getApiBaseUrl = () => {
  if (typeof import.meta !== 'undefined' && import.meta.env && import.meta.env.VITE_API_URL) {
    return import.meta.env.VITE_API_URL;
  }
  
  if (typeof window !== 'undefined') {
    const protocol = window.location.protocol;
    const hostname = window.location.hostname;
    if (hostname === 'localhost' || hostname === '127.0.0.1') {
      return `${protocol}//${hostname}:8080/api`;
    }
    
    return `${protocol}//${hostname}/api`;
  }
  
  return 'http://localhost:8080/api';
};

const API_BASE_URL = getApiBaseUrl();

interface ApiResponse<T> {
  status: string;
  data?: T;
  error?: string;
}

function getAuthToken(): string | null {
  if (typeof window !== 'undefined') {
    return localStorage.getItem('harbory_token');
  }
  return null;
}

function getAuthHeaders(): HeadersInit {
  const token = getAuthToken();
  const headers: HeadersInit = {
    'Content-Type': 'application/json',
  };
  
  if (token) {
    headers['Authorization'] = `Bearer ${token}`;
  }
  
  return headers;
}

async function fetchApi<T>(endpoint: string, options: RequestInit = {}): Promise<ApiResponse<T>> {
  try {
    const response = await fetch(`${API_BASE_URL}${endpoint}`, {
      ...options,
      credentials: 'include',
      headers: {
        ...getAuthHeaders(),
        ...options.headers,
      },
    });
    
    if (response.status === 401) {
      if (typeof window !== 'undefined') {
        localStorage.removeItem('harbory_token');
        window.location.href = '/login';
      }
      throw new Error('Unauthorized');
    }
    
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

export const systemApi = {
  getStats: () => fetchApi<any>('/system/stats')
};

export const authApi = {
  login: async (password: string) => {
    return fetchApi<{ token: string; message: string }>('/auth/login', {
      method: 'POST',
      body: JSON.stringify({ password }),
    });
  },
  logout: async () => {
    return fetchApi<{ message: string }>('/auth/logout', {
      method: 'POST',
    });
  },
  verify: () => fetchApi<{ message: string }>('/auth/verify'),
  changePassword: async (oldPassword: string, newPassword: string) => {
    return fetchApi<{ message: string }>('/auth/change-password', {
      method: 'POST',
      body: JSON.stringify({ old_password: oldPassword, new_password: newPassword }),
    });
  },
};
