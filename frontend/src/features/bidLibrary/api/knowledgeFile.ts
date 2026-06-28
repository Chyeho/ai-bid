import request from '@/api/request';
import type { BaseResponse, PageResponse } from '@/api/types';
import type {
   KnowledgeFile,
   KnowledgeQueryConfig,
   KnowledgeFileUpdateRequest,
} from '../types';

export const knowledgeFileApi = {
   getList: (
      params: KnowledgeQueryConfig
   ): Promise<PageResponse<KnowledgeFile>> => {
      return request.get('/api/knowledge-files', { params });
   },

   upload: (formData: FormData): Promise<BaseResponse<KnowledgeFile>> => {
      return request.post('/api/knowledge-files/upload', formData, {
         headers: { 'Content-Type': 'multipart/form-data' },
      });
   },

   update: (
      fileId: number,
      data: KnowledgeFileUpdateRequest
   ): Promise<BaseResponse<KnowledgeFile>> => {
      const formData = new FormData();
      if (data.fileName !== undefined) {
         formData.append('fileName', data.fileName);
      }
      if (data.category !== undefined) {
         formData.append('category', data.category);
      }
      if (data.applicableScope !== undefined) {
         formData.append('applicableScope', data.applicableScope);
      }
      if (data.description !== undefined) {
         formData.append('description', data.description ?? '');
      }
      if (data.status !== undefined) {
         formData.append('status', String(data.status));
      }
      return request.put(`/api/knowledge-files/${fileId}`, formData, {
         headers: { 'Content-Type': 'multipart/form-data' },
      });
   },

   updateStatus: (
      fileId: number,
      status: number
   ): Promise<BaseResponse<KnowledgeFile>> => {
      return request.patch(`/api/knowledge-files/${fileId}/status`, { status });
   },

   delete: (fileId: number): Promise<BaseResponse<void>> => {
      return request.delete(`/api/knowledge-files/${fileId}`);
   },

   download: (fileId: number): Promise<Blob> => {
      return request.get(`/api/knowledge-files/${fileId}/download`, {
         responseType: 'blob',
      });
   },

   preview: (fileId: number): Promise<Blob> => {
      return request.get(`/api/knowledge-files/${fileId}/preview`, {
         responseType: 'blob',
      });
   },
};
