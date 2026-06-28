import React from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { Result, Button, Typography } from 'antd';
import { useStyles } from './style';
import PdfPreview from './components/PDFPreview/PdfPreview';
import type { PdfPreviewRef } from './components/PDFPreview/PdfPreview';
import BidAnalysis from './components/BidAnalysis/BidAnalysis';
import { useAuditTask } from './hooks/useAuditTask';
import { auditDetailOptions } from './api/auditDetail';
import { useQuery } from '@tanstack/react-query';
import { Loading } from '@/components/Loading/Loading';

const { Text } = Typography;

export const DetailPage: React.FC = () => {
   const { styles } = useStyles();
   const { id: bidId } = useParams<{ id: string }>();
   const navigate = useNavigate();
   const pdfPreviewRef = React.useRef<PdfPreviewRef>(null);
   const handleLocateIssuePage = React.useCallback((page: number, highlightText?: string, fallbackTokens?: string[]) => {
      pdfPreviewRef.current?.jumpToPage(page, highlightText, fallbackTokens);
   }, []);

   const {
      data: bidData,
      isLoading,
      isError,
   } = useQuery({
      ...auditDetailOptions.bidDetail(Number(bidId)),
      enabled: !!bidId && !isNaN(Number(bidId)),
   });

   const {
      taskId,
      startAudit,
      isStarting,
      isAuditing,
      currentStage,
      elapsedSeconds,
      issues,
      isComplete,
      summary,
      error,
   } = useAuditTask(bidId ? Number(bidId) : undefined);

   if (isLoading) {
      return <Loading loading={isLoading} />;
   }

   if (isError || !bidData) {
      return (
         <div className={styles.detailContainer}>
            <Text type='secondary'>暂无项目详细信息或加载失败</Text>
         </div>
      );
   }

   if (error) {
      return (
         <div style={{ padding: '50px' }}>
            <Result
               status='error'
               title='审核任务异常'
               subTitle={error}
               extra={[
                  <Button
                     type='primary'
                     key='console'
                     onClick={() =>
                        startAudit({ bidId: Number(bidId), webSearchEnabled: false, forceRefresh: true })
                     }
                  >
                     重试
                  </Button>,

                  <Button key='back' onClick={() => navigate(-1)}>
                     返回列表
                  </Button>,
               ]}
            />
         </div>
      );
   }

   return (
      <div className={styles.container}>
         <div className={styles.mainContent}>
            <PdfPreview
               ref={pdfPreviewRef}
               fileUrl={`/api/bid-documents/${bidData.id}/download`}
               fileType={bidData.fileType}
               isComplete={isComplete}
            />

            <BidAnalysis
               onAudit={(options) =>
                  startAudit({
                     bidId: bidData.id,
                     webSearchEnabled: !!options?.webSearchEnabled,
                     forceRefresh: !!options?.forceRefresh,
                  })
               }
               projectId={bidData.projectId}
               bidId={bidData.id}
               taskId={taskId}
               isStarting={isStarting}
               isAuditing={isAuditing}
               elapsedSeconds={elapsedSeconds}
               issues={issues}
               isComplete={isComplete}
               currentStage={currentStage}
               summary={summary}
               onLocateIssuePage={handleLocateIssuePage}
               currentFileName={bidData.fileName || bidData.bidName}
               currentFileId={bidData.id}
            />
         </div>
      </div>
   );
};
