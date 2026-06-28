import React from 'react';
import { Tabs, Input, Select, Button } from 'antd';
import { SearchOutlined } from '@ant-design/icons';
import { CATEGORY_MAP } from '@/features/bidAudit/types';
import { useStyles } from '../style';
import { useIsMobile } from '@/hooks/useMediaQuery';

import type { IssueQueryParams } from '../types';

type FilterFields = Pick<IssueQueryParams, 'severity' | 'category' | 'keyword'>;

interface IssueTableFilterProps {
   severity: string;
   category: string;
   keyword: string;
   onChange: (values: Partial<FilterFields>) => void;
   onReset: () => void;
}

export const IssueTableFilter: React.FC<IssueTableFilterProps> = ({
   severity,
   category,
   keyword,
   onChange,
   onReset,
}) => {
   const { styles } = useStyles();
   const isMobile = useIsMobile();

   return (
      <div className={styles.filterBar}>
         <Tabs
            activeKey={severity}
            onChange={(val) => onChange({ severity: val })}
            items={[
               { key: 'all', label: '全部' },
               { key: 'critical', label: '严重' },
               { key: 'warning', label: '一般' },
               { key: 'info', label: '提示' },
            ]}
            style={{ marginBottom: -16 }}
            size='small'
         />

         <div className={styles.filterControls}>
            <Input
               placeholder='搜索问题关键词...'
               prefix={<SearchOutlined />}
               value={keyword}
               onChange={(e) => onChange({ keyword: e.target.value })}
               style={{
                  width: isMobile ? '100%' : 200,
                  height: isMobile ? 40 : 32,
               }}
               allowClear
            />

            <Select
               placeholder='类型筛选'
               allowClear
               style={{
                  width: isMobile ? '100%' : 120,
                  height: isMobile ? 40 : 32,
               }}
               value={category === 'all' ? undefined : category}
               onChange={(val) => onChange({ category: val || 'all' })}
               options={Object.entries(CATEGORY_MAP).map(([key, val]) => ({
                  value: key,
                  label: val,
               }))}
            />

            <Button
               onClick={onReset}
               style={{
                  width: isMobile ? '100%' : 60,
                  height: isMobile ? 40 : 32,
               }}
            >
               重置
            </Button>
         </div>
      </div>
   );
};
