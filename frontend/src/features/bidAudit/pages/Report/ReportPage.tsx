import React, { useState, useMemo, useEffect } from 'react';
import { message, Spin } from 'antd';
import { useParams } from 'react-router-dom';

import { ReportHeader } from './components/ReportHeader';
import { ReportPreview } from './components/ReportPreview';
import { ReportSettings } from './components/ReportSettings';
import { useStyles } from './style';

import {
   MOCK_REPORT,
   parseMarkdownToSections,
   generateWordDocument,
   REPORT_SECTIONS,
} from './utils';

import { useCtrlWheelZoom } from '../../hooks/useCtrlWheelZoom';
import { getReport, generateReport } from './api/auditReport';
import type { Report } from './types';

// 是否使用 Mock 数据（开发阶段设为 true，联调时改为 false）
const USE_MOCK = false;

export const ReportPage: React.FC = () => {
   const { styles } = useStyles();
   const [messageApi, contextHolder] = message.useMessage();
   const { id: auditIdOrTaskId } = useParams<{ id: string }>();

   // ================= 状态管理 =================
   // 报告数据
   const [report, setReport] = useState<Report | null>(null);
   const [loading, setLoading] = useState<boolean>(false);

   // A区状态
   const [scale, setScale] = useState<number>(100);
   const [isExporting, setIsExporting] = useState<boolean>(false);

   // C区状态：默认全选模块，并设定默认文件名
   const [selectedSections, setSelectedSections] = useState<string[]>([
      ...REPORT_SECTIONS,
   ]);
   const [fileName, setFileName] = useState<string>(
      '审核报告_2026年度数字校园智慧基座建设项目.docx'
   );

   const previewContainerRef = useCtrlWheelZoom(setScale, {
      min: 50,
      max: 200,
      step: 10,
   });

   // ================= 数据获取 =================
   useEffect(() => {
      const fetchReport = async () => {
         if (USE_MOCK) {
            setReport(MOCK_REPORT);
            return;
         }

         if (!auditIdOrTaskId) {
            messageApi.error('缺少审核任务 ID');
            return;
         }

         setLoading(true);
         try {
            const data = await getReport(auditIdOrTaskId);
            // 后端可能返回业务失败但不抛异常，统一在这里兜底触发生成
            if (data?.docContent?.trim()) {
               setReport(data);
               return;
            }
            const generated = await generateReport(auditIdOrTaskId);
            if (generated?.docContent?.trim()) {
               setReport(generated);
               return;
            }
            setReport(null);
            messageApi.error('报告暂无内容，请稍后重试。');
         } catch (error) {
            console.error('获取/生成报告失败', error);
            setReport(null);
            messageApi.error('获取报告失败，请稍后重试。');
         } finally {
            setLoading(false);
         }
      };

      fetchReport();
   }, [auditIdOrTaskId, messageApi]);

   // ================= 核心计算 (难点1 联动渲染) =================
   const currentMarkdownContent = useMemo(() => {
      if (!report) return '';

      const docContent = report.docContent;
      // 解析完整 Markdown 为各模块字典
      const sectionsDict = parseMarkdownToSections(docContent);

      // 强制按照 REPORT_SECTIONS 原有顺序进行拼接 (防止用户乱点导致结构错乱)
      return REPORT_SECTIONS.filter((section) =>
         selectedSections.includes(section)
      )
         .map((section) => sectionsDict[section])
         .filter((content) => !!content)
         .join('\n\n---\n');
   }, [report, selectedSections]);

   // ================= 核心导出 (难点2 纯前端导出) =================
   const handleExportWord = async () => {
      if (!fileName.trim()) {
         messageApi.warning('请输入导出的文件名');
         return;
      }

      if (!report) {
         messageApi.warning('暂无报告数据可导出');
         return;
      }

      setIsExporting(true);
      try {
         const paperElement = document.querySelector(`.${styles.a4Paper}`);
         if (!paperElement) throw new Error('未找到可导出的报告内容');

         const htmlContent = paperElement.innerHTML;

         await generateWordDocument(
            htmlContent,
            fileName.endsWith('.docx') ? fileName : `${fileName}.docx`
         );
         messageApi.success('Word 文档导出成功！');
      } catch (error) {
         console.error('导出失败', error);
         messageApi.error('导出失败，请检查数据格式。');
      } finally {
         setIsExporting(false);
      }
   };

   // ================= 加载状态 =================
   if (loading) {
      return (
         <div className={styles.pageContainer}>
            {contextHolder}
            <div
               style={{
                  display: 'flex',
                  justifyContent: 'center',
                  alignItems: 'center',
                  height: '100vh',
               }}
            >
               <Spin size='large' tip='正在加载报告...' />
            </div>
         </div>
      );
   }

   return (
      <div className={styles.pageContainer}>
         {contextHolder}

         {/* 顶部 A 区 */}
         <ReportHeader
            scale={scale}
            onScaleChange={setScale}
            onExportWord={handleExportWord}
            isExporting={isExporting}
         />

         <main className={styles.mainContent}>
            {/* 左侧 B 区：A4 纸预览 */}
            <ReportPreview
               scale={scale}
               markdownContent={currentMarkdownContent}
               ref={previewContainerRef}
            />

            {/* 右侧 C 区：导出设置联动区 */}
            <ReportSettings
               selectedSections={selectedSections}
               onSelectionChange={setSelectedSections}
               fileName={fileName}
               onFileNameChange={setFileName}
            />
         </main>
      </div>
   );
};
