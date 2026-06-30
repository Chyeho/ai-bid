import { store } from '@/store';
import { logout } from '@/store/slices/authSlice';
import axios from 'axios';

const request = axios.create({
   baseURL: import.meta.env.VITE_API_BASE_URL,
   timeout: 30000,
});

let isLoggingOut = false;

// 请求拦截器：自动注入 Token
request.interceptors.request.use((config) => {
   const token =
      localStorage.getItem('token') || sessionStorage.getItem('token');

   if (token && config.headers) {
      config.headers.Authorization = `Bearer ${token}`;
   }

   return config;
});

// 响应拦截器：统一处理错误
request.interceptors.response.use(
   (response) => {
      // 对于下载文件等直接返回 Blob 的情况
      if (response.config.responseType === 'blob') {
         return response.data;
      }
      return response.data;
   },
   (error) => {
      const status = error.response?.status;
      if (status === 401 && !isLoggingOut) {
         isLoggingOut = true;

         console.error('登录过期，请重新登录');
         store.dispatch(logout());

         window.location.href = '/login';

         setTimeout(() => {
            isLoggingOut = false;
         }, 500);
      } else {
         console.error(error.response?.data?.message || '网络请求错误');
      }
      return Promise.reject(error);
   }
);

export default request;
