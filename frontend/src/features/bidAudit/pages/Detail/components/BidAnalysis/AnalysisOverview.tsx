import React, { useCallback, useState } from 'react';
import { Typography, Card, Button, Spin, Tag, Switch } from 'antd';
import { useStyles } from '../../style';
import type { AuditSummary } from '../../types';
import { useNavigate, useParams } from 'react-router-dom';

const { Text } = Typography;

interface AnalysisOverviewProps {
   onAudit: (options?: { webSearchEnabled: boolean; forceRefresh?: boolean }) => void;
   taskId: string | null;
   isStarting: boolean;
   isAuditing: boolean;
   elapsedSeconds: number;
   currentStage: string;
   isComplete: boolean;
   summary: AuditSummary;
}

export const AnalysisOverview: React.FC<AnalysisOverviewProps> = React.memo(
   ({
      onAudit,
      taskId,
      isStarting,
      isAuditing,
      elapsedSeconds,
      currentStage,
      isComplete,
      summary,
   }) => {
      const { theme } = useStyles();
      const { id: bidId } = useParams<{ id: string }>();
      const navigate = useNavigate();
      const [manualStarted, setManualStarted] = useState(false);
      const [webSearchEnabled, setWebSearchEnabled] = useState(false);

      const handleStartAudit = useCallback(() => {
         setManualStarted(true);
         onAudit({ webSearchEnabled, forceRefresh: false });
      }, [onAudit, webSearchEnabled]);

      const handleReAudit = useCallback(() => {
         setManualStarted(true);
         onAudit({ webSearchEnabled, forceRefresh: true });
      }, [onAudit, webSearchEnabled]);

      // 仅在“本轮任务创建中/执行中”锁定，完成后允许再次调整。
      const lockWebSwitch = isStarting || isAuditing;

      const formatElapsed = (seconds: number) => {
         const value = Math.max(0, seconds);
         const hours = Math.floor(value / 3600);
         const minutes = Math.floor((value % 3600) / 60);
         const secs = value % 60;
         if (hours > 0) {
            return `${hours.toString().padStart(2, '0')}:${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
         }
         return `${minutes.toString().padStart(2, '0')}:${secs.toString().padStart(2, '0')}`;
      };

      const renderStatus = () => {
         if (summary.critical > 0) {
            return <Text type='danger'>不通过</Text>;
         }
         if (summary.warning > 0) {
            return <Text type='warning'>通过(建议修改)</Text>;
         }
         return <Text type='success'>通过</Text>;
      };

      const renderContent = () => {
         if (isComplete && !isAuditing) {
            return (
                  <>
                     <div
                        style={{
                           display: 'flex',
                           alignItems: 'center',
                           justifyContent: 'space-between',
                        }}
                     >
                        <span
                           className='review-result'
                           style={{ fontWeight: 'bold' }}
                        >
                           审核结论：
                           {renderStatus()}
                        </span>

                        <Text type='secondary'>
                           本次用时 {formatElapsed(elapsedSeconds)}
                        </Text>
                        <div
                           style={{
                              display: 'flex',
                              alignItems: 'center',
                              gap: 8,
                           }}
                        >
                           <Text style={{ fontSize: 12, color: theme.colorTextTertiary }}>
                              联网搜索兜底
                           </Text>
                           <Switch
                              checked={webSearchEnabled}
                              onChange={setWebSearchEnabled}
                              disabled={lockWebSwitch}
                              checkedChildren='开'
                              unCheckedChildren='关'
                           />
                        </div>
                     </div>

                     <div
                        style={{
                           display: 'flex',
                           alignItems: 'center',
                           gap: '1rem',
                           paddingTop: '1rem',
                        }}
                     >
                        <Button
                           onClick={handleReAudit}
                           style={{ fontSize: '1.2rem', flex: 1 }}
                        >
                           重新审核
                        </Button>

                        <Button
                           disabled={!isComplete}
                           onClick={() =>
                              navigate(`/bidReview/issues/${bidId}`)
                           }
                           style={{ fontSize: '1.2rem', flex: 1 }}
                        >
                           报告详情
                        </Button>

                        <Button
                           disabled={!isComplete}
                           onClick={() =>
                              navigate(`/bidReview/report/${bidId}`)
                           }
                           style={{ fontSize: '1.2rem', flex: 1 }}
                        >
                           导出报告
                        </Button>
                     </div>
                  </>
            );
         }

         // 尚未创建任务：显示“开始审核”按钮
         if (!taskId && !isAuditing && !manualStarted) {
            return (
               <div
                  style={{
                     display: 'flex',
                     flexDirection: 'column',
                     gap: 12,
                  }}
               >
                  <Button
                     type='primary'
                     onClick={handleStartAudit}
                     loading={isStarting}
                     disabled={isAuditing}
                     style={{ fontSize: '1.2rem', width: '100%' }}
                  >
                     {isStarting ? '正在创建审核任务...' : isAuditing ? '审核进行中...' : '开始审核'}
                  </Button>
                  <div
                     style={{
                        display: 'flex',
                        alignItems: 'center',
                        justifyContent: 'space-between',
                     }}
                  >
                     <Text style={{ fontSize: 13 }}>联网搜索兜底</Text>
                     <Switch
                        checked={webSearchEnabled}
                        onChange={setWebSearchEnabled}
                        disabled={lockWebSwitch}
                        checkedChildren='开'
                        unCheckedChildren='关'
                     />
                  </div>

                  <Text type='secondary' style={{ fontSize: 13 }}>
                     点击“开始审核”后，系统会调用大模型对标书进行自动审查，
                     审核完成后将在下方展示审核结论和问题列表。
                  </Text>
               </div>
            );
         }

         // 审核进行中：显示任务状态信息
         return (
            <>
               <div
                  style={{
                     border: `1px solid ${theme.colorBorderSecondary}`,
                     borderRadius: 8,
                     padding: '10px 12px',
                     background: theme.colorBgContainer,
                     display: 'flex',
                     justifyContent: 'space-between',
                     alignItems: 'center',
                     gap: 12,
                  }}
               >
                  <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                     <Tag color='processing'>审核中</Tag>
                     <Text
                        type={isComplete ? 'success' : 'secondary'}
                        style={{ fontSize: 13 }}
                     >
                        <Spin size='small' style={{ marginRight: 8 }} />
                        {currentStage}
                     </Text>
                     <Text style={{ fontSize: 12, color: theme.colorTextTertiary }}>
                        已用时 {formatElapsed(elapsedSeconds)}
                     </Text>
                     <Text style={{ fontSize: 12, color: theme.colorTextTertiary }}>
                        联网搜索
                     </Text>
                     <Switch
                        checked={webSearchEnabled}
                        onChange={setWebSearchEnabled}
                        disabled={lockWebSwitch}
                        checkedChildren='开'
                        unCheckedChildren='关'
                        size='small'
                     />
                  </div>
                  <Button onClick={handleReAudit} disabled={isAuditing || isStarting}>
                     重新审核
                  </Button>
               </div>
            </>
         );
      };

      return (
         <div>
            <Card
               size='small'
               variant='borderless'
               style={{
                  background: theme.colorFillQuaternary,
                  marginBottom: 12,
                  transition: 'all 0.3s ease-in-out',
               }}
            >
               {renderContent()}
            </Card>
         </div>
      );
   }
);
