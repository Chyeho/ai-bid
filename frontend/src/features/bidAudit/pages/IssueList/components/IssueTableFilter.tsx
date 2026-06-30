import React from 'react';
import { Tabs, Input, Button } from 'antd';
import { SearchOutlined } from '@ant-design/icons';
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
               { key: 'high', label: '高风险' },
               { key: 'medium', label: '中风险' },
               { key: 'low', label: '低风险' },
               { key: 'info', label: '信息' },
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
