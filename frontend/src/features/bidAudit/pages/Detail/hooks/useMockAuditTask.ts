import { useState, useCallback, useRef } from 'react';
import type { AuditIssue } from '../types';

const mockIssuesPool: AuditIssue[] = [
   {
      issueNo: '1',
      category: 'legal',
      severity: 'critical',
      location: {
         pageNumber: 2,
         sectionName: '资质章节',
         context: '排他性条款',
      },
      description: '资质要求涉及排他性条款',
      suggestion: '修改建议：删除“仅限本省企业参与”的表述。',
      reference: '招标投标法第10条',
   },
   {
      issueNo: '2',
      category: 'budget',
      severity: 'warning',
      location: { pageNumber: 5, sectionName: '付款说明', context: '节点描述' },
      description: '付款节点描述不清晰',
      suggestion:
         '修改建议：建议明确二期款项支付的具体前置条件和验收标准，避免后期款项纠纷。',
      reference: '',
   },
   {
      issueNo: '3',
      category: 'demand',
      severity: 'info',
      location: { pageNumber: 8, sectionName: '财务明细', context: '排版' },
      description: '排版格式不规范',
      suggestion:
         '修改建议：建议将财务明细表格的字体统一为宋体小四，并保持行距一致。',
      reference: '',
   },
   {
      issueNo: '4',
      category: 'budget',
      severity: 'critical',
      location: { pageNumber: 12, sectionName: '预算清单', context: '总额' },
      description: '预算总额超出项目批复限额',
      suggestion:
         '修改建议：当前分项报价总和（520万）已超出原定项目批复预算（500万）。',
      reference: '',
   },
   {
      issueNo: '5',
      category: 'legal',
      severity: 'warning',
      location: { pageNumber: 15, sectionName: '违约责任', context: '条款' },
      description: '违约责任划分不对等',
      suggestion: '修改建议：建议补充采购方违约时的赔偿基准，以防范法务风险。',
      reference: '',
   },
   {
      issueNo: '6',
      category: 'demand',
      severity: 'warning',
      location: { pageNumber: 18, sectionName: '核心设备', context: '单价' },
      description: '核心设备单价偏离市场均价',
      suggestion:
         '修改建议：检测到单价较历史中标均价高出20%，建议复核报价合理性。',
      reference: '',
   },
];

export const useMockAuditTask = () => {
   const [taskId, setTaskId] = useState<string | null>(null);
   const [progress, setProgress] = useState(0);
   const [issues, setIssues] = useState<AuditIssue[]>([]);
   const [isComplete, setIsComplete] = useState(false);
   const [currentStage, setCurrentStage] = useState('准备开始...');
   const [isStarting, setIsStarting] = useState(false);

   const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);
   const delayRef = useRef<ReturnType<typeof setTimeout> | null>(null);

   const startAudit = useCallback((bidId: number) => {
      if (timerRef.current) {
         clearInterval(timerRef.current);
         timerRef.current = null;
      }
      if (delayRef.current) {
         clearTimeout(delayRef.current);
         delayRef.current = null;
      }

      setIsStarting(true);

      delayRef.current = setTimeout(() => {
         setTaskId(`MOCK-${bidId}-${Date.now()}`);
         setIsStarting(false);
         setProgress(0);
         setIssues([]);
         setIsComplete(false);
         setCurrentStage('正在提取文档内容与结构...');

         let currentProgress = 0;

         timerRef.current = setInterval(() => {
            currentProgress += 5;
            if (currentProgress > 100) currentProgress = 100;
            setProgress(currentProgress);

            const expectedIssuesCount = Math.min(
               Math.floor((currentProgress / 95) * mockIssuesPool.length),
               mockIssuesPool.length
            );

            setIssues(mockIssuesPool.slice(0, expectedIssuesCount));

            if (currentProgress < 15)
               setCurrentStage('正在提取文档内容与结构...');
            else if (currentProgress < 30)
               setCurrentStage('正在进行预算合规性审核...');
            else if (currentProgress < 45)
               setCurrentStage('正在进行格式规范性审核...');
            else if (currentProgress < 60)
               setCurrentStage('正在进行财务与价格审核...');
            else if (currentProgress < 85)
               setCurrentStage('正在进行法律风险深度审核...');
            else if (currentProgress >= 100) {
               setCurrentStage('所有审核完成，正在汇总结果...');
               setIsComplete(true);
               clearInterval(timerRef.current!);
               timerRef.current = null;
            }
         }, 400);
      }, 800);
   }, []);

   return {
      taskId,
      startAudit,
      isStarting,
      progress,
      currentStage,
      issues,
      isComplete,
      error: null,
      summary: {
         totalIssues: issues.length,
         critical: issues.filter((i) => i?.severity === 'critical').length,
         warning: issues.filter((i) => i?.severity === 'warning').length,
         info: issues.filter((i) => i?.severity === 'info').length,
      },
   };
};
