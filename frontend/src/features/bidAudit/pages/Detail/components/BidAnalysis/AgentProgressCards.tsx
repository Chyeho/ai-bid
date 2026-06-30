import React from 'react';
import { Progress, Tag, Card, Row, Col } from 'antd';
import {
  CheckCircleOutlined,
  LoadingOutlined,
  ClockCircleOutlined,
  CloseCircleOutlined,
  ExclamationCircleOutlined,
} from '@ant-design/icons';
import type { AgentProgress } from '@/types/audit';
import { AGENT_LABELS } from '@/types/audit';

interface Props {
  progresses: Map<string, AgentProgress>;
}

const STATUS_ICON: Record<string, React.ReactNode> = {
  running: <LoadingOutlined style={{ color: '#1890ff' }} />,
  completed: <CheckCircleOutlined style={{ color: '#52c41a' }} />,
  pending: <ClockCircleOutlined style={{ color: '#d9d9d9' }} />,
  failed: <CloseCircleOutlined style={{ color: '#f5222d' }} />,
};

const STATUS_COLOR: Record<string, string> = {
  running: '#1890ff',
  completed: '#52c41a',
  pending: '#d9d9d9',
  failed: '#f5222d',
};

/**
 * Multi-Agent 并行审查进度卡片。
 * 审查完成后整个组件卸载。
 */
const AgentProgressCards: React.FC<Props> = ({ progresses }) => {
  if (progresses.size === 0) return null;

  const agents = Array.from(progresses.values());

  return (
    <div style={{ marginBottom: 16 }}>
      <div style={{
        fontSize: 14,
        fontWeight: 600,
        marginBottom: 12,
        color: '#1a1a1a',
      }}>
        ⚡ {agents.length} 个 Agent 并行审查中
      </div>
      <Row gutter={[8, 8]}>
        {agents.map((agent) => {
          const percent = agent.clauses_total > 0
            ? Math.round((agent.clauses_done / agent.clauses_total) * 100)
            : 0;
          const label = AGENT_LABELS[agent.agent_id] || agent.agent_label || agent.agent_id;

          return (
            <Col span={12} key={agent.agent_id}>
              <Card
                size="small"
                style={{
                  borderRadius: 8,
                  borderLeft: `3px solid ${STATUS_COLOR[agent.status] || '#d9d9d9'}`,
                }}
                bodyStyle={{ padding: '10px 12px' }}
              >
                <div style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', marginBottom: 6 }}>
                  <span style={{ fontSize: 13, fontWeight: 500 }}>
                    {STATUS_ICON[agent.status]} {label}
                  </span>
                  {agent.raw_findings > 0 && (
                    <Tag color="orange" style={{ margin: 0, fontSize: 11 }}>
                      <ExclamationCircleOutlined /> {agent.raw_findings} 疑似
                    </Tag>
                  )}
                </div>
                <Progress
                  percent={percent}
                  size="small"
                  status={agent.status === 'failed' ? 'exception' : agent.status === 'completed' ? 'success' : 'active'}
                  format={() => `${agent.clauses_done}/${agent.clauses_total}`}
                  style={{ marginBottom: 0 }}
                />
              </Card>
            </Col>
          );
        })}
      </Row>
    </div>
  );
};

export default AgentProgressCards;
