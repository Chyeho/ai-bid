import React, { useMemo } from 'react';
import { FilterBar } from './components/FilterBar';
import { HistoryTable } from './components/HistoryTable';
import type { HistoryQueryParams, ReviewStatus } from './types';
import { useStyles } from './style';
import { useUrlState } from '@/hooks/useUrlState';
import { useQuery } from '@tanstack/react-query';
import { historyOptions } from './api/history';

export const HistoryPage: React.FC = () => {
   const { styles } = useStyles();

   // 维护完整的查询参数状态
   const [queryParams, setQueryParams] = useUrlState<HistoryQueryParams>({
      page: 1,
      size: 10,
      auditResult: 'all',
      projectName: '',
      fileCategory: undefined,
      auditUserName: '',
      startDate: '',
      endDate: '',
   });

   // 获取数据 (TanStack Query) - 恢复真实数据请求
   const { data: recordsData, isFetching } = useQuery(
      historyOptions.list(queryParams)
   );

   // Mock Data - 注释掉 Mock 数据
   // const { data: recordsData, isFetching } = useHistoryData(queryParams);

   // 获取统计数据 (查询条件变化时，它会自动重新请求)
   const { data: statsData } = useQuery(historyOptions.stats(queryParams));

   // 处理过滤栏查询
   const handleSearch = (filterValues: Partial<HistoryQueryParams>) => {
      setQueryParams({ ...filterValues, page: 1 });
   };

   // 处理过滤栏重置
   const handleReset = () => {
      setQueryParams({
         projectName: '',
         fileCategory: undefined,
         auditUserName: '',
         startDate: '',
         endDate: '',
         page: 1,
      });
   };

   // 处理分页器变化
   const handlePageChange = (page: number) => {
      setQueryParams({ page });
   };

   // 处理 Tab 状态切换
   const handleTabChange = (key: string) => {
      setQueryParams({ auditResult: key as ReviewStatus | 'all', page: 1 });
   };

   // 计算用于展示的统计结构
   const displayStats = useMemo(() => {
      const defaultStats = {
         all: 0,
         pass: 0,
         revise: 0,
         reject: 0,
         passRatePercent: '0.0',
      };
      if (!statsData?.statusList) return defaultStats;

      const getCount = (statusKey: string) =>
         statsData.statusList.find((item) => item.status === statusKey)
            ?.count || 0;

      const all = getCount('all');
      const pass = getCount('pass');
      const revise = getCount('revise');
      const reject = getCount('reject');

      const passRatePercent = all > 0 ? ((pass / all) * 100).toFixed(1) : '0.0';

      return { all, pass, revise, reject, passRatePercent };
   }, [statsData]);

   return (
      <div className={styles.pageContainer}>
         <FilterBar
            initialValues={queryParams}
            onSearch={handleSearch}
            onReset={handleReset}
         />

         <HistoryTable
            data={recordsData?.records || []}
            loading={isFetching}
            total={recordsData?.total || 0}
            currentPage={queryParams.page}
            pageSize={queryParams.size}
            activeTab={queryParams.auditResult as ReviewStatus | 'all'}
            stats={displayStats}
            onPageChange={handlePageChange}
            onTabChange={handleTabChange}
         />
      </div>
   );
};
