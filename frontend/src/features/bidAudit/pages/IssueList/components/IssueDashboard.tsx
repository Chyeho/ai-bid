import React from 'react';
import { useStyles } from '../style';
import { AuditResultCard } from '@/components/StatCard/AuditResultCard';
import type { AuditSummary } from '@/types/audit';
import {
   QuestionCircleFilled,
   CloseCircleFilled,
   WarningFilled,
   InfoOutlined,
   AlertFilled,
} from '@ant-design/icons';

interface IssueDashboardProps {
   summary?: AuditSummary;
}

export const IssueDashboard: React.FC<IssueDashboardProps> = ({ summary }) => {
   const { styles, theme } = useStyles();

   const stats = {
      total: summary?.totalIssues || 0,
      high: summary?.high || 0,
      medium: summary?.medium || 0,
      low: summary?.low || 0,
      info: summary?.info || 0,
   };

   const cards = [
      {
         label: '问题总数',
         value: stats.total,
         color: theme.colorPrimary,
         icon: <QuestionCircleFilled />,
      },
      {
         label: '高风险',
         value: stats.high,
         color: theme.colorError,
         icon: <CloseCircleFilled />,
      },
      {
         label: '中风险',
         value: stats.medium,
         color: '#fa8c16',
         icon: <AlertFilled />,
      },
      {
         label: '低风险',
         value: stats.low,
         color: theme.colorWarning,
         icon: <WarningFilled />,
      },
      {
         label: '信息',
         value: stats.info,
         color: theme.colorPrimary,
         icon: <InfoOutlined />,
      },
   ];

   return (
      <div className={styles.statsGrid}>
         {cards.map((card, idx) => (
            <AuditResultCard
               key={idx}
               label={card.label}
               value={card.value}
               color={card.color}
               icon={card.icon}
               labelFontSize={'1.3rem'}
               valueFontSize={'2rem'}
               style={{ letterSpacing: '1px' }}
            />
         ))}
      </div>
   );
};
