import request from '@/api/request';
import type { BaseResponse } from '@/api/types';
import type { Report } from '../types';

export const getReport = async (
   auditIdOrTaskId: string | number
): Promise<Report> => {
   const res = await request.get<unknown, BaseResponse<Report>>(
      `/api/audit-reports/${auditIdOrTaskId}`
   );

   return res.data;
};

export const generateReport = async (
   taskIdOrAuditId: string | number
): Promise<Report> => {
   const res = await request.post<unknown, BaseResponse<Report>>(
      `/api/audit-reports/${taskIdOrAuditId}/generate`
   );

   return res.data;
};
