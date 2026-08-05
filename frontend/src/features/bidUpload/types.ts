export type { BidDocument } from '@/types/audit';

export type FileCategory = '标书' | '合同';

export interface BidUploadQueryParams {
  id: number;
  fileCategory: FileCategory;
  bidName: string;
  supplierName: string;
  budgetAmount: string;
  version: number;
  projectId: number;
}
