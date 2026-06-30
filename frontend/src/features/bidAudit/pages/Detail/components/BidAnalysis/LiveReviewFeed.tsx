import React, { useEffect, useRef } from 'react';
import { Tag } from 'antd';
import {
  BulbOutlined,
  SearchOutlined,
  FileSearchOutlined,
  WarningOutlined,
  SendOutlined,
  MessageOutlined,
  PlayCircleOutlined,
  LinkOutlined,
} from '@ant-design/icons';
import type { TraceEvent } from '@/types/audit';
import { AGENT_LABELS } from '@/types/audit';

interface Props {
  events: TraceEvent[];
}

const DARK_GREEN = '#52c41a';
const LIGHT_GREEN = '#e8f5e9';

const EVENT_ICON: Record<string, React.ReactNode> = {
  turn_start: <PlayCircleOutlined style={{ color: DARK_GREEN }} />,
  agent_thought: <BulbOutlined style={{ color: '#fa8c16' }} />,
  tool_call: <SearchOutlined style={{ color: '#1677ff' }} />,
  tool_result: <FileSearchOutlined style={{ color: DARK_GREEN }} />,
  output_finding: <WarningOutlined style={{ color: DARK_GREEN }} />,
  agent_bus_send: <SendOutlined style={{ color: DARK_GREEN }} />,
  agent_bus_recv: <MessageOutlined style={{ color: DARK_GREEN }} />,
};

const EVENT_LABEL: Record<string, string> = {
  turn_start: '审查轮次',
  agent_thought: '推理',
  tool_call: '工具调用',
  tool_result: '工具结果',
  output_finding: '风险发现',
  agent_bus_send: '跨Agent通知',
  agent_bus_recv: '收到通知',
};

interface SearchSource {
  title?: string;
  url?: string;
  score?: string;
}

const isImportant = (type: string) => type === 'output_finding';

const LiveReviewFeed: React.FC<Props> = ({ events }) => {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [events.length]);

  if (events.length === 0) {
    return null;
  }

  return (
    <div style={{ marginBottom: 16 }}>
      <div style={{
        fontSize: 14,
        fontWeight: 600,
        marginBottom: 10,
        color: '#1a1a1a',
      }}>
        实时审查动态
      </div>
      <div style={{
        maxHeight: 420,
        overflowY: 'auto',
        background: '#fafafa',
        borderRadius: 8,
        padding: '8px 12px',
        border: '1px solid #f0f0f0',
      }}>
        {events.slice(-80).map((event, idx) => {
          const agentLabel = AGENT_LABELS[event.agent_name] || event.agent_name;
          const important = isImportant(event.event_type);
          const isThought = event.event_type === 'agent_thought';
          const isToolCall = event.event_type === 'tool_call';
          const isToolResult = event.event_type === 'tool_result';

          // tool_result: extract sources from payload
          const sources: SearchSource[] =
            isToolResult && event.payload?.sources
              ? (event.payload.sources as SearchSource[])
              : [];

          return (
            <div
              key={idx}
              style={{
                display: 'flex',
                gap: 8,
                padding: '5px 0',
                borderBottom: '1px solid #f5f5f5',
                fontSize: 12,
                lineHeight: '18px',
                background: important ? LIGHT_GREEN : undefined,
                borderRadius: important ? 4 : undefined,
                paddingLeft: important ? 8 : undefined,
                paddingRight: important ? 8 : undefined,
              }}
            >
              {/* Icon */}
              <span style={{ fontSize: 14, flexShrink: 0, marginTop: 1 }}>
                {EVENT_ICON[event.event_type] || <BulbOutlined style={{ color: DARK_GREEN }} />}
              </span>

              {/* Content */}
              <div style={{ flex: 1, minWidth: 0 }}>
                {/* Header row: agent + tag + turn */}
                <span style={{ display: 'flex', alignItems: 'center', gap: 4, marginBottom: 1 }}>
                  <span style={{ fontWeight: 500, color: '#595959' }}>
                    {agentLabel}
                  </span>
                  <Tag
                    color={important ? DARK_GREEN : undefined}
                    style={{
                      margin: 0,
                      fontSize: 10,
                      lineHeight: '16px',
                      padding: '0 4px',
                      color: important ? '#fff' : isThought ? '#d46b08' : isToolCall ? '#1677ff' : '#8c8c8c',
                      background: important ? DARK_GREEN : isThought ? '#fff7e6' : isToolCall ? '#e6f4ff' : '#f5f5f5',
                      border: important ? 'none' : isThought ? '1px solid #ffd591' : isToolCall ? '1px solid #91caff' : '1px solid #d9d9d9',
                    }}
                  >
                    {EVENT_LABEL[event.event_type] || event.event_type}
                  </Tag>
                  <span style={{ color: '#bfbfbf', fontSize: 10 }}>
                    T{event.turn}
                  </span>
                </span>

                {/* Summary */}
                {event.summary && !isToolResult && (
                  <div style={{
                    color: isThought ? '#8c6d00' : '#8c8c8c',
                    wordBreak: 'break-all',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    display: '-webkit-box',
                    WebkitLineClamp: 2,
                    WebkitBoxOrient: 'vertical',
                  }}>
                    {event.summary}
                  </div>
                )}

                {/* tool_result: summary + links */}
                {isToolResult && event.summary && (
                  <div style={{ color: '#8c8c8c', marginBottom: sources.length > 0 ? 4 : 0 }}>
                    {event.summary}
                  </div>
                )}
                {sources.length > 0 && (
                  <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
                    {sources.map((src, si) => (
                      <a
                        key={si}
                        href={src.url || '#'}
                        target="_blank"
                        rel="noopener noreferrer"
                        onClick={(e) => {
                          if (!src.url) e.preventDefault();
                        }}
                        style={{
                          display: 'flex',
                          alignItems: 'center',
                          gap: 4,
                          fontSize: 11,
                          color: '#1677ff',
                          textDecoration: 'none',
                          overflow: 'hidden',
                          textOverflow: 'ellipsis',
                          whiteSpace: 'nowrap',
                        }}
                        title={src.title || src.url}
                      >
                        <LinkOutlined style={{ flexShrink: 0, fontSize: 10 }} />
                        <span style={{ overflow: 'hidden', textOverflow: 'ellipsis' }}>
                          {src.title || src.url}
                        </span>
                        {src.score && (
                          <span style={{ color: '#bfbfbf', flexShrink: 0, fontSize: 10 }}>
                            [{src.score}]
                          </span>
                        )}
                      </a>
                    ))}
                  </div>
                )}
              </div>
            </div>
          );
        })}
        <div ref={bottomRef} />
      </div>
    </div>
  );
};

export default LiveReviewFeed;
