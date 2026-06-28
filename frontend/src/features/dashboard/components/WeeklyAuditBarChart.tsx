import React, { useEffect, useMemo, useRef } from 'react';
import * as echarts from 'echarts';
import { COLORS } from '@/theme/constants';
import { Typography } from 'antd';
import { useStyles } from '../style';
import type { AuditCountItem } from '../types';

const { Title } = Typography;

interface WeeklyAuditBarChartProps {
   data?: AuditCountItem[];
}

export const WeeklyAuditBarChart: React.FC<WeeklyAuditBarChartProps> = ({
   data,
}) => {
   const chartRef = useRef<HTMLDivElement>(null);
   const { styles } = useStyles();
   const chartData = Array.isArray(data) ? data : [];

   const option: echarts.EChartsOption = useMemo(
      () => ({
         grid: {
            top: 5,
            right: 15,
            left: 10,
            bottom: 0,
            containLabel: true,
         },
         tooltip: {
            trigger: 'axis',
            axisPointer: { type: 'shadow' },
            formatter: (params: any) => {
               const val = params[0];
               return `${val.name}<br/>
                    <span style="display:inline-block;margin-right:4px;border-radius:10px;width:10px;height:10px;background-color:${COLORS.primary};"></span>
                    审核数量: <b>${val.value}</b> 个`;
            },
         },
         xAxis: {
            type: 'value',
            max: 100,
            axisLine: { show: false },
            axisTick: { show: false },
            splitLine: {
               lineStyle: { type: 'dashed', color: COLORS.border },
            },
            axisLabel: { color: COLORS.textSecondary, fontSize: 12 },
         },
         yAxis: {
            type: 'category',
            data: chartData.map((item) => item.name),
            inverse: true,
            axisLine: { show: false },
            axisTick: { show: false },
            axisLabel: {
               color: COLORS.textSecondary,
               fontSize: 12,
               margin: 12,
            },
         },
         series: [
            {
               type: 'bar',
               data: chartData.map((item) => item.count),
               barWidth: 20,
               itemStyle: {
                  color: COLORS.primary,
               },
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
            本周审核统计
         </Title>
         <div ref={chartRef} style={{ width: '100%', height: 250 }} />
      </div>
   );
};
