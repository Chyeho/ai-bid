import request from '@/api/request';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import type { BaseResponse } from '@/api/types';
import type { BidUploadQueryParams, BidDocument } from '../types';

export const uploadBidDocument = async (
   params: BidUploadQueryParams,
   file: File
): Promise<BidDocument> => {
   const formData = new FormData();
   formData.append('file', file);

   const res = await request.post<unknown, BaseResponse<BidDocument>>(
      '/api/bid-documents/upload',
      formData,
      {
         params,
         headers: {
            'Content-Type': 'multipart/form-data',
         },
      }
   );

   return res.data;
};

export const useUploadBidMutation = () => {
   const queryClient = useQueryClient();

   return useMutation({
      mutationFn: ({
         params,
         file,
      }: {
         params: BidUploadQueryParams;
         file: File;
      }) => uploadBidDocument(params, file),
      onSuccess: async (_, variables) => {
         await Promise.all([
            queryClient.invalidateQueries({ queryKey: ['dashboardList'] }),
            queryClient.invalidateQueries({ queryKey: ['auditList'] }),
            queryClient.invalidateQueries({ queryKey: ['auditListWithParams'] }),
            queryClient.invalidateQueries({
               queryKey: ['projectVersions', variables.params.projectId ?? null],
            }),
         ]);
      },
   });
};
