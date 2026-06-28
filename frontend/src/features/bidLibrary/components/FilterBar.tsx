import { Select, Button } from 'antd';
import { ReloadOutlined } from '@ant-design/icons';
import { useStyles } from '../style';
import { useIsMobile } from '@/hooks/useMediaQuery';
import { ResponsiveRangePicker } from '@/components/ResponsiveRangePicker/ResponsiveRangePicker';
import type { Dayjs } from 'dayjs';
import type { KnowledgeQueryConfig } from '../types';

interface FilterBarProps {
   applicableScopeFilter: NonNullable<KnowledgeQueryConfig['applicableScope']>;
   onApplicableScopeChange: (
      value: NonNullable<KnowledgeQueryConfig['applicableScope']>
   ) => void;
   statusFilter: NonNullable<KnowledgeQueryConfig['status']>;
   onStatusChange: (value: NonNullable<KnowledgeQueryConfig['status']>) => void;
   dateRange: [Dayjs | null, Dayjs | null] | null;
   onDateRangeChange: (value: [Dayjs | null, Dayjs | null] | null) => void;
   onReset: () => void;
}

export function FilterBar({
   applicableScopeFilter,
   onApplicableScopeChange,
   statusFilter,
   onStatusChange,
   dateRange,
   onDateRangeChange,
   onReset,
}: FilterBarProps) {
   const { styles } = useStyles();
   const isMobile = useIsMobile();

   return (
      <div className={styles.filterBar}>
         <div className={`${styles.filterItem}`}>
            <span className={styles.filterLabel}>适用范围</span>
            <Select
               placeholder='采购类/工程类/通用'
               value={applicableScopeFilter || undefined}
               onChange={onApplicableScopeChange}
               allowClear
               style={{
                  width: isMobile ? '100%' : 180,
               }}
               options={[
                  { value: 'procurement', label: '采购类' },
                  { value: 'engineering', label: '工程类' },
                  { value: 'general', label: '通用' },
               ]}
            />
         </div>

         <div className={styles.filterItem}>
            <span className={styles.filterLabel}>状态</span>
            <Select
               placeholder='启用/停用'
               value={statusFilter || undefined}
               onChange={onStatusChange}
               allowClear
               style={{ width: isMobile ? '100%' : 150 }}
               options={[
                  { value: 'enabled', label: '启用' },
                  { value: 'disabled', label: '停用' },
               ]}
            />
         </div>

         <div className={styles.filterItem}>
            <Button icon={<ReloadOutlined />} onClick={onReset}>
               重置
            </Button>
         </div>

         <div className={styles.filterItem}>
            <span className={styles.filterLabel}>上传时间</span>
            <ResponsiveRangePicker
               value={dateRange}
               onChange={onDateRangeChange}
            />
         </div>
      </div>
   );
}
