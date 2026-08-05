import request from '@/api/request';
import type { BaseResponse } from '@/api/types';
import type { LoginParams, LoginResponse, RegisterParams } from '../types';

export const loginApi = {
   login: (data: LoginParams): Promise<BaseResponse<LoginResponse>> => {
      return request.post('/api/auth/login', data);
   },

   logout: (): Promise<BaseResponse<any>> => {
      return request.post('/api/auth/logout');
   },

   refreshToken: (
      refreshToken: string
   ): Promise<BaseResponse<{ token: string }>> => {
      return request.post(
         '/api/auth/refresh',
         {},
         {
            headers: {
               Authorization: `Bearer ${refreshToken}`,
            },
         }
      );
   },

   register: (data: RegisterParams): Promise<BaseResponse<any>> => {
      return request.post('/api/auth/register', data);
   },
};
