import { useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { auditIssuesListOptions } from '../api/auditIssuesList';
import type { IssueQueryParams, AuditIssue } from '../types';
export const useAuditIssuesList = (
   taskId: string,
   params: IssueQueryParams
) => {
   const { data, isLoading, isFetching, error } = useQuery({
      ...auditIssuesListOptions.result(taskId),
      enabled: !!taskId,
   });

   const filteredIssues = useMemo(() => {
      const rawIssues = data?.issues ?? [];
      if (!rawIssues.length) return [];

      const allIssues: AuditIssue[] = rawIssues as AuditIssue[];

      return allIssues.filter((issue: AuditIssue) => {
         const matchSeverity =
            !params.severity ||
            params.severity === 'all' ||
            issue.severity === params.severity;

         const matchCategory =
            !params.category ||
            params.category === 'all' ||
            issue.category === params.category;

         const keyword = params.keyword?.toLowerCase().trim();

         const matchKeyword =
            !keyword ||
            issue.description?.toLowerCase().includes(keyword) ||
            issue.suggestion?.toLowerCase().includes(keyword) ||
            issue.location?.context?.toLowerCase().includes(keyword) ||
            issue.location?.context?.toLowerCase().includes(keyword) ||
            issue.location?.sectionName?.toLowerCase().includes(keyword);

         return matchSeverity && matchCategory && matchKeyword;
      });
   }, [data?.issues, params.severity, params.category, params.keyword]);

   const paginatedIssues = useMemo(() => {
      const page = params.page ?? 1;
      const size = params.size ?? 10;
      const startIndex = (page - 1) * size;
      const endIndex = startIndex + size;

      return filteredIssues.slice(startIndex, endIndex);
   }, [filteredIssues, params.page, params.size]);

   return {
      rawResult: data,
      summary: data?.summary,
      issues: paginatedIssues,
      total: filteredIssues.length,
      isLoading,
      isFetching,
      error,
   };
};
