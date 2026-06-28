export type KnowledgeFileCategory = 'regulation' | 'price' | 'supplier' | 'contract' | 'case' | 'other';
export type KnowledgeFileStatus = 0 | 1 | 2;
export type ApplicableScope = 'procurement' | 'engineering' | 'general';

export interface KnowledgeFile {
   id: number;
   file_name: string;
   file_path: string;
   file_size: number;
   file_type: string;
   category: KnowledgeFileCategory;
   tags: string;
   description: string;
   status: KnowledgeFileStatus;
   version: number;
   chunk_count: number;
   upload_user_id: number;
   upload_user_name: string;
   upload_time: string;
   update_time: string;
   applicable_scope: ApplicableScope;
}

export interface KnowledgeFileListRequest {
   category?: string;
   page: number;
   size: number;
   applicable_scope?: string;
   status?: string;
   start_time?: string;
   end_time?: string;
}

export interface KnowledgeFileSearchRequest {
   keyword: string;
   page: number;
   size: number;
}

export interface KnowledgeFileUploadRequest {
   file: File;
   file_name: string;
   category: KnowledgeFileCategory;
   applicable_scope: ApplicableScope;
   description?: string;
}

export interface KnowledgeFileUpdateRequest {
   file_name?: string;
   category?: KnowledgeFileCategory;
   applicable_scope?: ApplicableScope;
   description?: string;
   status?: KnowledgeFileStatus;
}

export interface KnowledgeFileListResponse {
   list: KnowledgeFile[];
   total: number;
   page: number;
   size: number;
}

export const CategoryMap: Record<KnowledgeFileCategory, string> = {
   regulation: '制度文件',
   price: '价格标准',
   supplier: '供应商名录',
   contract: '合同模板',
   case: '案例库',
   other: '其他',
};

export const ApplicableScopeMap: Record<ApplicableScope, string> = {
   procurement: '采购类',
   engineering: '工程类',
   general: '通用',
};
