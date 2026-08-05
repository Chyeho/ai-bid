import React, { useEffect, useMemo, useRef } from 'react';
import * as echarts from 'echarts';
import { COLORS } from '@/theme/constants';
import { Typography } from 'antd';
import { useStyles } from '../style';
import type { IssueChartItem } from '../types';

// 基于全局配置生成渐变调色盘
const PIE_COLORS = [
   COLORS.primary, // 学校绿
   COLORS.primaryHover, // 中绿
   COLORS.success, // 成功绿
   '#81C784', // 补充渐变过渡色
   '#A5D6A7', // 补充渐变过渡色
   COLORS.primaryLight, // 浅绿色
];

interface IssueTypePieChartProps {
   data?: IssueChartItem[];
}

const { Title } = Typography;

export const IssueTypePieChart: React.FC<IssueTypePieChartProps> = ({
   data,
}) => {
   const chartRef = useRef<HTMLDivElement>(null);
   const { styles } = useStyles();
   const chartData = Array.isArray(data) ? data : [];

   const option: echarts.EChartsOption = useMemo(
      () => ({
         color: PIE_COLORS,
         tooltip: {
            trigger: 'item',
         },
         legend: {
            bottom: '5',
            left: 'center',
            icon: 'square',
            textStyle: {
               fontSize: 12,
               color: COLORS.textSecondary,
            },
         },
         series: [
            {
               name: '问题类型',
               type: 'pie',
               radius: ['42%', '62%'],
               center: ['50%', '44%'],
               avoidLabelOverlap: false,
               padAngle: 2,
               label: {
                  show: false,
                  position: 'center',
               },
               labelLine: {
                  show: false,
               },
               data: chartData,
            },
         ],
      }),
      [chartData]
   );

   useEffect(() => {
      if (!chartRef.current) return;
      const chart = echarts.init(chartRef.current);

      const resizeObserver = new ResizeObserver(() => {
         chart.resize();
      });

      resizeObserver.observe(chartRef.current);

      return () => {
         resizeObserver.disconnect();
         chart.dispose();
      };
   }, []);

   useEffect(() => {
      if (!chartRef.current) return;
      const chart = echarts.getInstanceByDom(chartRef.current);
      if (!chart) return;

      chart.setOption(option);
   }, [option]);

   return (
      <div className={styles.chartCard}>
         <Title level={4} style={{ marginTop: 5 }}>
            问题类型分布
         </Title>

         <span style={{ fontSize: 12 }}>合规性、法律法规、采购需求等维度分布。</span>

         <div ref={chartRef} style={{ width: '100%', height: 250 }} />
      </div>
   );
};
