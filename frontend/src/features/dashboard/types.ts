export interface ProjectParams {
   id: number;
   projectName: string;
   supplierName?: string;
}

export interface Tender {
   id: number;
   fileName: string;
   filePath: string;
   fileSize: number;
   fileType: 'word' | 'pdf';
   fileCategory: '标书' | '合同';
   bidName: string;
   supplierName: string;
   budgetAmount: number;
   pageCount: number;
   parseStatus: number; // 0:Pending, 1:Processing, 2:Completed, 3:Failed
   uploadUserId: number;
   uploadTime: string;
   version: number;
   projectId: number;
   auditResult?: string | null;
}

export interface auditTask {
   id: number;
   taskId: string;
   bidId: number;
   taskStatus: number;
   auditResult: string;
   issueCount: number;
   criticalCount: number;
   warningCount: number;
   infoCount: number;
   startTime: string;
   endTime: string;
   auditUserId: number;
   createTime: string;
}

export interface auditReport {
   id: number;
   auditId: number;
   docContent: string;
   version: number;
   generateTime: string;
}

// 该项目下的标书及其审核报告列表
export interface BidDetail {
   tender: Tender;
   auditTask: auditTask;
   auditReport: auditReport;
}

export interface ProjectItem {
   id: number;
   userId: number;
   projectName: string;
   supplierName: string;
   parseStatus: number; // 0未审核, 1已审核
   latestVersion: number;
   createTime: string;
   updateTime: string;
   auditResult?: string | null;
   tenders: BidDetail[];
}

export interface DistributionResponse {
   budget: number;
   legal: number;
   demand: number;
}

export interface AuditCount {
   周一: number;
   周二: number;
   周三: number;
   周四: number;
   周五: number;
   周六: number;
   周日: number;
}

export interface IssueChartItem {
   name: string;
   value: number;
}

export interface AuditCountItem {
   name: string;
   count: number;
}
