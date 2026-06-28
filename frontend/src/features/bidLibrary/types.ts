import type { PageParams } from '@/api/types';

export type KnowledgeFileCategory =
   | 'regulation'
   | 'price'
   | 'supplier'
   | 'contract'
   | 'case'
   | 'other';
export type KnowledgeFileStatus = 0 | 1; // 0停用 1启用
export type ApplicableScope = 'procurement' | 'engineering' | 'general';

// 基本类型
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
   uploadUserId: number;
   uploadUserName: string;
   uploadTime: string;
   updateTime: string;
   applicableScope: ApplicableScope;
}

// 查询
export interface KnowledgeQueryConfig extends PageParams {
   category?: KnowledgeFileCategory | 'all';
   keyword?: string;
   applicableScope?: ApplicableScope | '';
   status?: 'enabled' | 'disabled' | '';
   startDate?: string;
   endDate?: string;
}

// --- 更新/编辑 ---
export interface KnowledgeFileUpdateRequest {
   fileName?: string;
   category?: KnowledgeFileCategory;
   applicableScope?: ApplicableScope;
   description?: string;
   status?: KnowledgeFileStatus;
}

// 分类
export const CategoryMap: Record<KnowledgeFileCategory, string> = {
   regulation: '制度文件',
   price: '价格标准',
   supplier: '供应商名录',
   contract: '合同模板',
   case: '案例库',
   other: '其他',
};

// 适用范围
export const ApplicableScopeMap: Record<ApplicableScope, string> = {
   procurement: '采购类',
   engineering: '工程类',
   general: '通用',
};
