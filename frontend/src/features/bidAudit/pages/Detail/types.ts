export type AuditCategory = 'budget' | 'legal' | 'demand';

export const CATEGORY_MAP: Record<AuditCategory, string> = {
   budget: '预算合规性',
   legal: '政策合法性',
   demand: '需求合规性',
};

export interface CreateTaskParams {
   bidId: number;
   enabledChecks: string[];
   webSearchEnabled?: boolean;
   forceRefresh?: boolean;
}

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
   dimension?: string;
   description: string;
   location: AuditLocation;
   suggestion: string;
   reference: string;
   anchorQuote?: string;
   anchorPage?: number;
   anchorSection?: string;
   anchorTokens?: string[];
   anchorCharsRange?: number[];
}

export interface AuditResult {
   taskId: string;
   auditResult: string;
   summary: AuditSummary;
   issues: AuditIssue[];
}

export interface ChatCitation {
   text?: string;
   documentName?: string;
   pageNumber?: number | string;
   content?: string;
   meta?: {
      document_id?: number | string;
      file_id?: number | string;
      fileId?: number | string;
      file_name?: string;
      fileName?: string;
      source_type?: string;
      sourceType?: string;
      pageNumber?: number | string;
      sectionName?: string;
   };
}

export interface SendChatRequest {
   projectId: number;
   bidId: number;
   content: string;
   saveToKnowledgeBase: boolean;
   mode?: 'default' | 'supplement';
}

export interface SendChatResponse {
   content: string;
   citations: ChatCitation[];
}

export interface FetchChatHistoryParams {
   projectId: number;
   bidId: number;
   days?: number;
}

export interface ChatHistoryItem {
   id: number;
   projectId: number;
   bidId: number;
   role: 'user' | 'assistant';
   content: string;
   createTime: string;
}
