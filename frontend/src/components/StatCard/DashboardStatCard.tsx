import React from 'react';
import { Typography } from 'antd';
import { createStyles } from 'antd-style';

const { Text } = Typography;

const useStyles = createStyles(({ css, token }) => ({
   cardContainer: css`
      background: ${token.colorBgContainer};
      border-radius: 6px;
      padding: 1.5rem;
      display: flex;
      flex-direction: column;
      gap: 1rem;
      transition: box-shadow 0.3s ease-in-out, transform 0.2s ease-in;
      box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);

      &:hover {
         transform: translateY(-2px);
         box-shadow: 0 6px 15px rgba(0, 0, 0, 0.2);
      }
   `,
}));

export interface StatCardProps {
   label: string;
   value: string | number;
   color: string;
   labelFontSize?: string | number;
   valueFontSize?: string | number;
   icon: React.ReactNode;
   style?: React.CSSProperties;
}

export const DashboardStatCard: React.FC<StatCardProps> = ({
   label,
   value,
   color,
   labelFontSize,
   valueFontSize,
   icon,
   style,
}) => {
   const { styles, theme } = useStyles();

   return (
      <div
         className={styles.cardContainer}
         style={{
            ...style,
         }}
      >
         <div style={{ color: color, fontSize: 16 }}>{icon}</div>

         <div
            style={{
               flex: 1,
               display: 'flex',
               alignItems: 'center',
               gap: '1rem',
            }}
         >
            <Text
               style={{
                  color: theme.colorText,
                  fontSize: labelFontSize,
               }}
            >
               {label}
            </Text>

            <div
               style={{
                  fontSize: valueFontSize,
                  fontWeight: 'bold',
                  color: color,
               }}
            >
               {value}
            </div>
         </div>
      </div>
   );
};
