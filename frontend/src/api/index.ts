// SecurePass API 客户端
import axios from 'axios';

const API_BASE_URL = process.env.NODE_ENV === 'development' 
  ? 'http://localhost:3000/api' 
  : '/api';

const api = axios.create({
  baseURL: API_BASE_URL,
  timeout: 10000,
  headers: {
    'Content-Type': 'application/json',
  },
});

// 密码管理 API
export const passwordApi = {
  // 获取所有密码
  getPasswords: () => api.get<PasswordEntry[]>('/passwords'),
  
  // 添加密码
  addPassword: (password: NewPassword) => api.post<PasswordEntry>('/passwords', password),
  
  // 更新密码
  updatePassword: (id: number, password: UpdatePassword) => api.put<PasswordEntry>(`/passwords/${id}`, password),
  
  // 删除密码
  deletePassword: (id: number) => api.delete(`/passwords/${id}`),
  
  // 搜索密码
  searchPasswords: (query: string) => api.get<PasswordEntry[]>(`/passwords/search?q=${query}`),
};

// 认证 API
export const authApi = {
  // 登录
  login: (credentials: LoginRequest) => api.post<LoginResponse>('/auth/login', credentials),
  
  // 登出
  logout: () => api.post('/auth/logout'),
  
  // 修改密码
  changePassword: (request: ChangePasswordRequest) => api.post('/auth/change-password', request),
  
  // 验证密码强度
  validatePassword: (password: string) => api.post('/auth/validate-password', { password }),
};

// 导入导出 API
export const importExportApi = {
  // 导出密码
  exportPasswords: (format: 'json' | 'csv' = 'json') => api.get(`/export/passwords?format=${format}`, {
    responseType: 'blob',
  }),
  
  // 导入密码
  importPasswords: (file: File, format: 'json' | 'csv' = 'json') => {
    const formData = new FormData();
    formData.append('file', file);
    formData.append('format', format);
    return api.post('/import/passwords', formData, {
      headers: {
        'Content-Type': 'multipart/form-data',
      },
    });
  },
  
  // 创建备份
  createBackup: () => api.post('/backup/create'),
  
  // 恢复备份
  restoreBackup: (backupId: string) => api.post(`/backup/restore/${backupId}`),
};

// 通知 API
export const notificationApi = {
  showNotification: (message: string, type: 'success' | 'error' | 'warning' = 'success') => {
    // 在实际应用中，这里会调用系统通知API
    console.log(`[${type.toUpperCase()}] ${message}`);
  },
};

// 数据类型定义
export interface PasswordEntry {
  id: number;
  site_name: string;
  site_url: string;
  username: string;
  encrypted_password: string;
  encrypted_notes?: string;
  created_at: string;
  updated_at: string;
  last_accessed?: string;
  access_count: number;
  tags: string[];
  category: string;
  priority: number;
  metadata?: string;
  checksum?: string;
}

export interface NewPassword {
  site_name: string;
  site_url: string;
  username: string;
  password: string;
  notes?: string;
  tags?: string[];
  category?: string;
  priority?: number;
}

export interface UpdatePassword {
  site_name?: string;
  site_url?: string;
  username?: string;
  password?: string;
  notes?: string;
  tags?: string[];
  category?: string;
  priority?: number;
}

export interface LoginRequest {
  master_password: string;
  remember_me?: boolean;
}

export interface LoginResponse {
  success: boolean;
  session_id?: string;
  error?: string;
}

export interface ChangePasswordRequest {
  old_password: string;
  new_password: string;
}

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  error?: string;
  timestamp: string;
}

// 错误处理
export const handleApiError = (error: any) => {
  if (error.response) {
    // 服务器响应了，但状态码不在 2xx 范围内
    console.error('API Error:', error.response.data);
    notificationApi.showNotification(
      error.response.data.error || '请求失败',
      'error'
    );
  } else if (error.request) {
    // 请求已发出，但没有收到响应
    console.error('Network Error:', error.request);
    notificationApi.showNotification('网络连接失败', 'error');
  } else {
    // 在设置请求时发生错误
    console.error('Request Error:', error.message);
    notificationApi.showNotification('请求设置失败', 'error');
  }
  throw error;
};

// 请求拦截器
api.interceptors.request.use(
  (config) => {
    // 在这里可以添加认证token等
    const token = localStorage.getItem('auth_token');
    if (token) {
      config.headers.Authorization = `Bearer ${token}`;
    }
    return config;
  },
  (error) => {
    return Promise.reject(error);
  }
);

// 响应拦截器
api.interceptors.response.use(
  (response) => {
    return response;
  },
  (error) => {
    handleApiError(error);
    return Promise.reject(error);
  }
);

export default api;