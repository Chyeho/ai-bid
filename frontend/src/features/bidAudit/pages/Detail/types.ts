/**
 * 审核详情页类型 — 全部从共享类型库引入。
 * 本文件保留用于向后兼容，所有类型定义在 @/types/audit.ts。
 */
export type {
  AuditIssue,
  AuditSummary,
  AuditStatus,
  AuditResult,
  AuditLocation,
  CreateTaskParams,
  SendChatRequest,
  SendChatResponse,
  ChatCitation,
  ChatHistoryItem,
  FetchChatHistoryParams,
} from '@/types/audit';

import type { BidDocument } from '@/types/audit';
export type BidDetail = BidDocument;
