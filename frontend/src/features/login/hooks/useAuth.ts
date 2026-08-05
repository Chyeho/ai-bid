import { useMutation } from '@tanstack/react-query';
import { loginApi } from '../api/login';
import type { LoginParams, RegisterParams } from '../types';

export const useLoginMutation = () => {
   return useMutation({
      mutationFn: (data: LoginParams) => loginApi.login(data),
   });
};

export const useRegisterMutation = () => {
   return useMutation({
      mutationFn: (data: RegisterParams) => loginApi.register(data),
   });
};
