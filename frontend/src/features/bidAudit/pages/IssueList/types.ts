import type { PageParams } from '@/api/types';

export type AuditCategory = 'budget' | 'legal' | 'demand';

export const CATEGORY_MAP: Record<AuditCategory, string> = {
   budget: '预算合规性',
   legal: '政策合法性',
   demand: '需求合规性',
};

export interface BidDetail {
   id: number;
   fileName: string;
   filePath: string;
   fileType: string;
   fileCategory: 'bid' | 'contract';
   bidName: string;
   supplierName: string;
   budgetAmount: string;
   uploadTime: string;
   version: number;
   projectId: number;
   auditorName: string;
}

export interface AuditStatus {
   taskId: string;
   status: string;
   stage: string;
   progress: number;
   issueCount: number;
   failedStages: string[];
   totalFileCount: number;
   pendingFileCount: number;
   processingFileCount: number;
   failedFileCount: number;
}

export interface AuditSummary {
   totalIssues: number;
   critical: number;
   warning: number;
   info: number;
}

export interface AuditLocation {
   pageNumber: number;
   sectionName: string;
   context: string;
}

export interface AuditIssue {
   issueNo: string;
   severity: string;
   category: string;
   description: string;
   location: AuditLocation;
   suggestion: string;
   reference: string;
}

export interface AuditResult {
   taskId: string;
   auditResult: string;
   summary: AuditSummary;
   issues: AuditIssue[];
}

export interface IssueQueryParams extends PageParams {
   severity?: string | 'all';
   category?: string | 'all';
   keyword?: string;
}
