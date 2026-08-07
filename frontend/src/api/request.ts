import { store } from '@/store';
import { logout, switchTenant } from '@/store/slices/authSlice';
import axios from 'axios';
import type { AxiosError } from 'axios';

const request = axios.create({
  baseURL: import.meta.env.VITE_API_BASE_URL,
  timeout: 30000,
});

let isLoggingOut = false;
let isRefreshing = false;

// 请求拦截器：自动注入 Token
request.interceptors.request.use((config) => {
  const token =
    localStorage.getItem('token') || sessionStorage.getItem('token');

  if (token && config.headers) {
    config.headers.Authorization = `Bearer ${token}`;
  }

  return config;
});

// 响应拦截器：按飞书《前端多租户交接文档》错误码细分处理
request.interceptors.response.use(
  (response) => {
    if (response.config.responseType === 'blob') {
      return response.data;
    }
    return response.data;
  },
  async (error: AxiosError<{ code?: number; msg?: string; error_code?: string }>) => {
    const status = error.response?.status;
    const errorCode = error.response?.data?.error_code;
    const errorMsg = error.response?.data?.msg;

    // ── 401：认证相关，按 error_code 细分 ──────────────────────────
    if (status === 401) {
      // TENANT_SESSION_STALE：不使用旧 token 重放，刷新会话或等切租户完成
      if (errorCode === 'TENANT_SESSION_STALE') {
        console.warn('租户会话已过期，请重新切换租户');
        if (!isLoggingOut) {
          isLoggingOut = true;
          store.dispatch(logout());
          window.location.href = '/login';
          setTimeout(() => { isLoggingOut = false; }, 500);
        }
        return Promise.reject(error);
      }

      // AUTH_REQUIRED / AUTH_INVALID：尝试刷新一次
      if ((errorCode === 'AUTH_REQUIRED' || errorCode === 'AUTH_INVALID') && !isRefreshing) {
        isRefreshing = true;
        try {
          const refreshToken = localStorage.getItem('refreshToken') || sessionStorage.getItem('refreshToken');
          if (refreshToken) {
            // 用 refresh token 换新 token
            const resp = await axios.post(
              `${import.meta.env.VITE_API_BASE_URL}/api/auth/refresh`,
              {},
              { headers: { Authorization: `Bearer ${refreshToken}` } }
            );
            if (resp.data?.code === 200 && resp.data?.data) {
              const newToken = resp.data.data.token;
              const storage = localStorage.getItem('token') ? localStorage : sessionStorage;
              storage.setItem('token', newToken);
              // 重放原请求
              const originalConfig = error.config;
              if (originalConfig?.headers) {
                originalConfig.headers.Authorization = `Bearer ${newToken}`;
              }
              isRefreshing = false;
              return request(originalConfig!);
            }
          }
          // refresh 失败，清登录状态
          throw new Error('refresh failed');
        } catch (refreshError) {
          isRefreshing = false;
          if (!isLoggingOut) {
            isLoggingOut = true;
            store.dispatch(logout());
            window.location.href = '/login';
            setTimeout(() => { isLoggingOut = false; }, 500);
          }
          return Promise.reject(error);
        }
      }

      // 其他 401（无 error_code 或未知）：默认清登录
      if (!isLoggingOut) {
        isLoggingOut = true;
        console.error('登录过期，请重新登录');
        store.dispatch(logout());
        window.location.href = '/login';
        setTimeout(() => { isLoggingOut = false; }, 500);
      }
      return Promise.reject(error);
    }

    // ── 403：权限相关，保留登录状态，提示无权限 ─────────────────────
    if (status === 403 && errorCode === 'TENANT_ROLE_FORBIDDEN') {
      console.warn('当前租户角色无权限执行此操作');
      // 不 logout，不跳转，由 UI 层提示用户
      return Promise.reject(error);
    }

    // ── 400：TENANT_REQUIRED — 引导选择或创建租户 ──────────────────
    if (status === 400 && errorCode === 'TENANT_REQUIRED') {
      console.warn('需要先选择或创建租户');
      // 不自行补 tenant_id，由 UI 层引导
      return Promise.reject(error);
    }

    // ── 404：TENANT_NOT_FOUND — 刷新租户列表 ────────────────────────
    if (status === 404 && errorCode === 'TENANT_NOT_FOUND') {
      console.warn('租户不存在，请刷新租户列表');
      return Promise.reject(error);
    }

    // ── 其他错误 ─────────────────────────────────────────────────────
    console.error(errorMsg || '网络请求错误');
    return Promise.reject(error);
  }
);

export default request;
