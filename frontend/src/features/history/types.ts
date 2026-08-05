import type { PageParams } from '@/api/types';

export type ReviewStatus = 'pass' | 'revise' | 'reject';

export type FileCategory = '标书' | '合同';

// 历史记录数据模型
export interface HistoryRecord {
   id: number;
   taskId: string;
   bidId: number;
   projectName: string;
   fileCategory: FileCategory;
   supplierName: string;
   budgetAmount: number;
   taskStatus: number;
   auditResult: ReviewStatus;
   issueCount: number;
   criticalCount: number;
   warningCount: number;
   infoCount: number;
   startTime: string;
   endTime: string;
   auditUserName: string;
   createTime: string;
}

// 查询参数配置
export interface HistoryQueryParams extends PageParams {
   auditResult?: ReviewStatus | 'all';
   projectName?: string;
   fileCategory?: FileCategory;
   auditUserName?: string;
   startDate?: string;
   endDate?: string;
}

// 历史记录统计数据
export interface HistoryStatistics {
   statusList: Array<{
      status: ReviewStatus | 'all';
      label: string;
      count: number;
   }>;
}
