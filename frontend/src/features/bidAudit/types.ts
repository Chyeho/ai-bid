import type { PageParams } from '@/api/types';

export type AuditCategory = 'budget' | 'legal' | 'demand';

export const CATEGORY_MAP: Record<AuditCategory, string> = {
   budget: '预算合规性',
   legal: '政策合法',
   demand: '需求合规',
};

export const ParseStatus = {
   Pending: 0,
   Processing: 1,
   Completed: 2,
   Failed: 3,
} as const;

export type ParseStatusType = (typeof ParseStatus)[keyof typeof ParseStatus];

export type FileCategory = '标书' | '合同';

export interface AuditIssue {
   id: string;
   type: 'critical' | 'warning' | 'info';
   category: AuditCategory;
   page: number;
   title: string;
   suggestion: string;
}

export interface AuditStats {
   critical: number;
   warning: number;
   info: number;
}

export interface AuditListItem {
   projectId: number;
   projectName: string;
   createTime: string;
   latestVersion: number;
   fileCategory: FileCategory;
   supplierName: string;
   auditorName: string;
}

export interface ProjectItem {
   id: number;
   fileName: string;
   filePath: string;
   fileSize: number;
   fileType: string;
   fileCategory: FileCategory;
   bidName: string;
   supplierName: string;
   budgetAmount: number;
   pageCount: number;
   parseStatus: ParseStatusType;
   uploadUserId: number;
   uploadTime: string;
   version: number;
   projectId: number;
   auditorName: string;
   auditResult?: string | null;
}

export interface AuditListQueryParams extends PageParams {
   bidName?: string;
   fileCategory?: FileCategory;
   status?: number;
   uploadStartTime?: string;
   uploadEndTime?: string;
}
