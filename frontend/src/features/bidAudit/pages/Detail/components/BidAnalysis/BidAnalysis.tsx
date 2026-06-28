import React from 'react';
import { Tabs } from 'antd';
import { useStyles } from '../../style';
import { AnalysisOverview } from './AnalysisOverview';
import { AnalysisList } from './AnalysisList';
import type { AuditIssue, AuditSummary } from '../../types';
import { AuditAssistant } from '../AuditAssistant/AuditAssistant';

interface BidAnalysisProps {
   onAudit: (options?: { webSearchEnabled: boolean; forceRefresh?: boolean }) => void;
   projectId: number;
   bidId: number;
   taskId: string | null;
   isStarting: boolean;
   isAuditing: boolean;
   days?: number;
   elapsedSeconds: number;
   issues: AuditIssue[];
   isComplete: boolean;
   currentStage: string;
   summary: AuditSummary;
   onLocateIssuePage: (page: number, highlightText?: string, fallbackTokens?: string[]) => void;
   currentFileName?: string;
   currentFileId?: number;
}

const BidAnalysis: React.FC<BidAnalysisProps> = ({
   onAudit,
   projectId,
   bidId,
   taskId,
   isStarting,
   isAuditing,
   elapsedSeconds,
   issues,
   isComplete,
   currentStage,
   summary,
   onLocateIssuePage,
   currentFileName,
   currentFileId,
}) => {
   const { styles } = useStyles();
   const rightPanelRef = React.useRef<HTMLDivElement>(null);
   const [overlayHost, setOverlayHost] = React.useState<HTMLElement | null>(null);
   const [activeTab, setActiveTab] = React.useState<'result' | 'chat'>('result');

   React.useEffect(() => {
      setOverlayHost(rightPanelRef.current);
   }, []);

   const tabItems = [
      {
         key: 'result',
         label: '审核结果',
         children: (
            <div
               style={{
                  display: 'flex',
                  flexDirection: 'column',
                  height: '100%',
                  gap: 10,
                  position: 'relative',
               }}
            >
               <AnalysisOverview
                  onAudit={onAudit}
                  taskId={taskId}
                  isStarting={isStarting}
                  isAuditing={isAuditing}
                  elapsedSeconds={elapsedSeconds}
                  currentStage={currentStage}
                  isComplete={isComplete}
                  summary={summary}
               />

               <AnalysisList
                  issues={issues}
                  isComplete={isComplete}
                  onLocateIssuePage={onLocateIssuePage}
                  overlayHost={overlayHost}
                  currentFileName={currentFileName}
                  currentFileId={currentFileId}
               />
            </div>
         ),
      },

      {
         key: 'chat',
         label: '审核助手',
         children: (
            <div style={{ height: '100%', paddingBottom: 12 }}>
               <AuditAssistant projectId={projectId} bidId={bidId} />
            </div>
         ),
      },
   ];

   return (
      <div ref={rightPanelRef} className={styles.rightPanel}>
         <Tabs
            activeKey={activeTab}
            onChange={(key) => setActiveTab(key as 'result' | 'chat')}
            items={tabItems}
            style={{ width: '100%', height: '100%' }}
            indicator={{ size: (origin) => origin - 40 }}
         />
      </div>
   );
};

export default React.memo(BidAnalysis);
