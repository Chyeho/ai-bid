/**
 * 问题列表页类型 — 全部从共享类型库引入。
 */
export type {
  AuditIssue,
  AuditSummary,
  AuditResult,
  AuditLocation,
  AuditStatus,
} from '@/types/audit';

import type { BidDocument } from '@/types/audit';
export type BidDetail = BidDocument;

/** 问题筛选参数 */
export interface IssueQueryParams {
  page: number;
  size: number;
  severity?: string;
  category?: string;
  keyword?: string;
}
