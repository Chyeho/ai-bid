import React, { useState, useCallback, useEffect } from 'react';
import { Tabs, Typography, Button, Space } from 'antd';
import {
  SearchOutlined,
  MessageOutlined,
  ReloadOutlined,
  ExportOutlined,
  EyeOutlined,
} from '@ant-design/icons';
import { useNavigate, useParams } from 'react-router-dom';
import { useStyles } from '../../style';
import { AnalysisList } from './AnalysisList';
import PipelineProgress from './PipelineProgress';
import ClauseActivityMap from './ClauseActivityMap';
import ReasoningDrawer from './ReasoningDrawer';
import type {
  AuditIssue, AuditSummary, AgentProgress,
  TraceEvent as TraceEventType,
  PhaseEvent, StatsEvent, FindingAddedEvent,
} from '@/types/audit';
import { AuditAssistant } from '../AuditAssistant/AuditAssistant';

const { Text } = Typography;

type DetailTab = 'process' | 'results' | 'chat';

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
  /** BBox-based 精确高亮回调（AnalysisList → DetailPage → PdfPreview） */
  onLocateBboxes?: (page: number, bboxes: import('../../components/PDFPreview/PdfPreview').BBoxData[]) => void;
  agentProgresses?: Map<string, AgentProgress>;
  liveFeedEvents?: TraceEventType[];
  phaseEvent?: PhaseEvent | null;
  statsEvent?: StatsEvent | null;
  liveFindings?: FindingAddedEvent[];
}

/**
 * 右侧审核面板 — 3 标签页：
 * 1. 审核过程 — 条款动态地图（实时追踪审查进度）
 * 2. 审核结果 — 风险发现列表
 * 3. 智能问答 — AI 对话助手
 */
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
  onLocateBboxes,
  agentProgresses,
  liveFeedEvents,
  phaseEvent,
  statsEvent,
  liveFindings,
}) => {
  const { styles } = useStyles();
  const navigate = useNavigate();
  const { id: routeBidId } = useParams<{ id: string }>();
  const rightPanelRef = React.useRef<HTMLDivElement>(null);
  const [activeTab, setActiveTab] = useState<DetailTab>(() => {
    // 有进行中的任务 → 默认看过程；否则看结果
    return isAuditing ? 'process' : 'results';
  });
  const [drawerIssue, setDrawerIssue] = useState<AuditIssue | null>(null);

  // 审核开始时自动切到"审核过程"；审核完成时自动切到"审核结果"
  useEffect(() => {
    if (isAuditing && !isComplete) {
      setActiveTab('process');
    }
  }, [isAuditing]);

  useEffect(() => {
    if (isComplete) {
      setActiveTab('results');
    }
  }, [isComplete]);

  const handleIssueClick = useCallback((issue: AuditIssue) => {
    setDrawerIssue(issue);
  }, []);

  const handleCloseDrawer = useCallback(() => {
    setDrawerIssue(null);
  }, []);

  const tabItems = [
    // ── Tab 1: 审核过程 ─────────────────────────────────
    {
      key: 'process' as DetailTab,
      label: (
        <span>
          <EyeOutlined /> 审核过程
        </span>
      ),
      children: (
        <div style={{ display: 'flex', flexDirection: 'column', height: '100%', gap: 0 }}>
          {(isAuditing || isComplete) && (
            <ClauseActivityMap
              traceEvents={liveFeedEvents || []}
              liveFindings={liveFindings || []}
              issues={issues}
              phaseEvent={phaseEvent || null}
              statsEvent={statsEvent || null}
              agentProgresses={agentProgresses || new Map()}
              isAuditing={isAuditing}
              isComplete={isComplete}
              elapsedSeconds={elapsedSeconds}
              onLocateIssuePage={onLocateIssuePage}
            />
          )}

          {(isAuditing || isComplete) && (
            <PipelineProgress currentStage={currentStage} isComplete={isComplete} />
          )}

          {/* 未开始时显示"开始审核" */}
          {!taskId && !isAuditing && (
            <div style={{ padding: '8px 6px' }}>
              <Button
                type="primary"
                block
                loading={isStarting}
                onClick={() => onAudit({ webSearchEnabled: false, forceRefresh: false })}
              >
                {isStarting ? '创建中...' : '开始审核'}
              </Button>
            </div>
          )}
        </div>
      ),
    },

    // ── Tab 2: 审核结果 ─────────────────────────────────
    {
      key: 'results' as DetailTab,
      label: (
        <span>
          <SearchOutlined /> 审核结果
        </span>
      ),
      children: (
        <div style={{ display: 'flex', flexDirection: 'column', height: '100%', gap: 0 }}>
          <div style={{ flex: 1, minHeight: 0, overflow: 'auto' }}>
            <AnalysisList
              issues={issues}
              isComplete={isComplete}
              onLocateIssuePage={onLocateIssuePage}
              currentFileName={currentFileName}
              currentFileId={currentFileId}
              onIssueClick={handleIssueClick}
              taskId={taskId}
              onLocateBboxes={onLocateBboxes}
            />
          </div>

          {/* 审核完成：底部操作栏 */}
          {isComplete && (
            <div style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'space-between',
              padding: '8px 6px',
              borderTop: '1px solid #f0f0f0',
              gap: 8,
            }}>
              <Text type="secondary" style={{ fontSize: 13 }}>
                {summary.totalIssues > 0
                  ? `共 ${summary.totalIssues} 条发现`
                  : '未发现合规风险'}
              </Text>
              <Space size={8}>
                <Button
                  size="small"
                  icon={<ReloadOutlined />}
                  onClick={() => onAudit({ webSearchEnabled: false, forceRefresh: true })}
                >
                  重新审核
                </Button>
                <Button
                  size="small"
                  icon={<ExportOutlined />}
                  onClick={() => navigate(`/bidReview/report/${routeBidId}`)}
                >
                  导出报告
                </Button>
              </Space>
            </div>
          )}
        </div>
      ),
    },

    // ── Tab 3: 智能问答 ─────────────────────────────────
    {
      key: 'chat' as DetailTab,
      label: (
        <span>
          <MessageOutlined /> 智能问答
        </span>
      ),
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
        onChange={(key) => setActiveTab(key as DetailTab)}
        items={tabItems}
        style={{ width: '100%', height: '100%' }}
        tabBarStyle={{ width: '100%' }}
      />

      <ReasoningDrawer
        issue={drawerIssue}
        open={drawerIssue !== null}
        onClose={handleCloseDrawer}
        onLocatePage={onLocateIssuePage}
      />
    </div>
  );
};

export default React.memo(BidAnalysis);
