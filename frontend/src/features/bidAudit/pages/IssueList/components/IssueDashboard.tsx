import React from 'react';
import { useStyles } from '../style';
import { AuditResultCard } from '@/components/StatCard/AuditResultCard';
import type { AuditSummary } from '../types';
import {
   QuestionCircleFilled,
   CloseCircleFilled,
   WarningFilled,
   InfoOutlined,
} from '@ant-design/icons';

interface IssueDashboardProps {
   summary?: AuditSummary; 
}

export const IssueDashboard: React.FC<IssueDashboardProps> = ({ summary }) => {
   const { styles, theme } = useStyles();

   const stats = {
      total: summary?.totalIssues || 0,
      critical: summary?.critical || 0,
      warning: summary?.warning || 0,
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
         label: '严重风险',
         value: stats.critical,
         color: theme.colorError,
         icon: <CloseCircleFilled />,
      },
      {
         label: '一般风险',
         value: stats.warning,
         color: theme.colorWarning,
         icon: <WarningFilled />,
      },
      {
         label: '提示建议',
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
