import React, { useRef, useEffect, useCallback } from 'react';
import { Typography } from 'antd';
import { RobotOutlined } from '@ant-design/icons';
import { useStyles } from './style';
import { MessageBubble } from './MessageBubble';
import { ChatInput } from './ChatInput';
import type { ChatMessage } from '../../hooks/useAiChat';

const { Text } = Typography;

interface ChatWindowProps {
  messages: ChatMessage[];
  isLoading: boolean;
  isHistoryLoading?: boolean;
  onSend: (content: string) => void;
  onClear: () => void;
}

export const ChatWindow: React.FC<ChatWindowProps> = ({
  messages,
  isLoading,
  isHistoryLoading,
  onSend,
  onClear: _onClear,
}) => {
  const { styles } = useStyles();

  const bottomRef = useRef<HTMLDivElement>(null);
  const hasInteracted = useRef(false);

  useEffect(() => {
    if (isHistoryLoading) return;
    bottomRef.current?.scrollIntoView({
      behavior: hasInteracted.current ? 'smooth' : 'auto',
    });
  }, [messages.length, isLoading, isHistoryLoading]);

  const handleSend = useCallback(
    (content: string) => {
      hasInteracted.current = true;
      onSend(content);
    },
    [onSend]
  );

  return (
    <div className={styles.container}>
      {/* ── Message list ── */}
      <div className={styles.messageList}>
        {isHistoryLoading ? (
          <div className={styles.centered}>
            <Text type="secondary">加载中...</Text>
          </div>
        ) : messages.length === 0 ? (
          <div className={styles.centered}>
            <div className={styles.emptyIcon}>
              <RobotOutlined />
            </div>
            <Text type="secondary">开始提问吧</Text>
          </div>
        ) : (
          messages.map((m: ChatMessage, idx: number) => (
            <MessageBubble key={m.id || `msg-${idx}`} message={m} />
          ))
        )}

        {/* ── Typing indicator (hide once AI starts responding) ── */}
        {isLoading && (() => {
          const lastMsg = messages[messages.length - 1];
          return !lastMsg || lastMsg.role !== 'assistant';
        })() && (
          <div className={styles.typing}>
            <div className={styles.typingAvatar}>
              <RobotOutlined />
            </div>
            <div className={styles.typingBubble}>
              <span />
              <span />
              <span />
            </div>
          </div>
        )}

        <div ref={bottomRef} />
      </div>

      {/* ── Footer / Input ── */}
      <footer className={styles.footer}>
        <ChatInput
          onSend={handleSend}
          disabled={isLoading}
        />
      </footer>
    </div>
  );
};
