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

export interface BidDocument {
   id: number;
   fileName: string;
   filePath: string;
   fileSize: number;
   fileType: string;
   fileCategory: FileCategory;
   bidName: string;
   supplierName: string;
   budgetAmount: string;
   pageCount: number;
   parseStatus: 0 | 1 | 2 | 3;
   uploadUserId: number;
   uploadTime: string;
   version: number;
   projectId: number;
   auditorName: string;
}
