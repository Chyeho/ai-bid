import type { Dayjs } from 'dayjs';

export type ReviewStatus = 'passed' | 'revision' | 'rejected';

export interface HistoryRecord {
   id: string;
   projectName: string;
   bidType: '采购标书' | '工程标书';
   supplierName: string;
   reviewer: string;
   reviewTime: string;
   status: ReviewStatus;
   issueCount: number;
}

export interface HistoryQueryConfig {
   page: number;
   pageSize: number;
   projectName?: string;
   dateRange?: [Dayjs, Dayjs] | null;
   bidType?: string;
   reviewer?: string;
   status?: ReviewStatus | 'all';
}
