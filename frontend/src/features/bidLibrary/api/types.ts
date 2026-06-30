/**
 * 知识库 API 类型 — 统一使用 camelCase。
 */
export type KnowledgeFileCategory = 'regulation' | 'price' | 'supplier' | 'contract' | 'case' | 'other';
export type KnowledgeFileStatus = 0 | 1 | 2;
export type ApplicableScope = 'procurement' | 'engineering' | 'general';

export interface KnowledgeFile {
  id: number;
  fileName: string;
  filePath: string;
  fileSize: number;
  fileType: string;
  category: KnowledgeFileCategory;
  tags: string;
  description: string;
  status: KnowledgeFileStatus;
  version: number;
  chunkCount: number;
  uploadUserId: number;
  uploadUserName: string;
  uploadTime: string;
  updateTime: string;
  applicableScope: ApplicableScope;
}

export interface KnowledgeFileListRequest {
  category?: string;
  page: number;
  size: number;
  applicableScope?: string;
  status?: string;
  startTime?: string;
  endTime?: string;
}

export interface KnowledgeFileSearchRequest {
  keyword: string;
  page: number;
  size: number;
}

export interface KnowledgeFileUploadRequest {
  file: File;
  fileName: string;
  category: KnowledgeFileCategory;
  applicableScope: ApplicableScope;
  description?: string;
}

export interface KnowledgeFileUpdateRequest {
  fileName?: string;
  category?: KnowledgeFileCategory;
  applicableScope?: ApplicableScope;
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
