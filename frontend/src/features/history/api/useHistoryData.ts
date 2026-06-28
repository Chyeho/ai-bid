import { useQuery } from '@tanstack/react-query';
import type { HistoryRecord, HistoryQueryParams } from '../types';
import dayjs from 'dayjs';
import type { PageResponse } from '@/api/types';

// 生成 25 条 Mock 数据以支持分页测试
const MOCK_DATA: HistoryRecord[] = Array.from({ length: 25 }).map(
   (_, index) => {
      const isPass = index % 3 === 0;
      const isRevise = index % 3 === 1;

      // 模拟问题数量计算
      const totalIssues = isPass
         ? 0
         : isRevise
         ? 3 + (index % 5)
         : 8 + (index % 5);
      const critical = isPass ? 0 : Math.floor(totalIssues / 3);
      const warning = isPass ? 0 : Math.floor(totalIssues / 2);
      const info = totalIssues - critical - warning;

      // 模拟时间节点
      const baseTime = dayjs().subtract(index, 'day');

      return {
         id: 1000 + index, // 后端 Long id
         taskId: `TASK-${2025000 + index}`, // 后端 String taskId
         bidId: 5000 + index, // 后端 Long bidId
         projectName: `${
            ['校园网络改造', '图书馆设备采购', '实验室通风系统', '宿舍楼维修'][
               index % 4
            ]
         }项目(第${index + 1}期)`,
         fileCategory: index % 2 === 0 ? '标书' : '合同',
         supplierName: `${
            ['中科信息', '神州数码', '建工集团', '三建工程'][index % 4]
         }有限公司`,
         budgetAmount: 500000 + index * 50000,
         taskStatus: 2,
         auditResult: isPass ? 'pass' : isRevise ? 'revise' : 'reject',
         issueCount: totalIssues,
         criticalCount: critical,
         warningCount: warning,
         infoCount: info,
         startTime: baseTime.subtract(2, 'hour').format('YYYY-MM-DD'),
         endTime: baseTime.format('YYYY-MM-DD'),
         createTime: baseTime.subtract(1, 'day').format('YYYY-MM-DD'),
         auditUserName: ['张会计', '李主任', '王科长', '刘干事'][index % 4],
      };
   }
);

// 模拟异步请求
const fetchHistory = async (
   params: HistoryQueryParams
): Promise<PageResponse<HistoryRecord>> => {
   return new Promise((resolve) => {
      setTimeout(() => {
         let filteredData = [...MOCK_DATA];

         // 1. 过滤状态
         if (params.auditResult && params.auditResult !== 'all') {
            filteredData = filteredData.filter(
               (item) => item.auditResult === params.auditResult
            );
         }
         // 2. 过滤项目名称
         if (params.projectName) {
            filteredData = filteredData.filter((item) =>
               item.projectName.includes(params.projectName!)
            );
         }
         // 3. 过滤标书类型
         if (params.fileCategory) {
            filteredData = filteredData.filter(
               (item) => item.fileCategory === params.fileCategory
            );
         }
         // 4. 过滤审核人
         if (params.auditUserName) {
            filteredData = filteredData.filter((item) =>
               item.auditUserName.includes(params.auditUserName!)
            );
         }
         // 5. 过滤时间
         if (params.startDate && params.endDate) {
            filteredData = filteredData.filter(
               (item) =>
                  item.endTime >= params.startDate! &&
                  item.endTime <= params.endDate! + ' 23:59:59'
            );
         }

         // 分页计算
         const total = filteredData.length;
         const startIndex = (params.page - 1) * params.size;
         const paginatedData = filteredData.slice(
            startIndex,
            startIndex + params.size
         );

         resolve({
            records: paginatedData,
            total,
         });
      }, 600);
   });
};

export const useHistoryData = (params: HistoryQueryParams) => {
   return useQuery({
      queryKey: ['historyRecords', params],
      queryFn: () => fetchHistory(params),
      placeholderData: (previousData) => previousData,
   });
};
