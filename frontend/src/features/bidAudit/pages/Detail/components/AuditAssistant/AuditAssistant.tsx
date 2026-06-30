import React from 'react';
import { useAiChat } from '../../hooks/useAiChat';
import { ChatWindow } from './ChatWindow';

interface AuditAssistantProps {
  projectId: number;
  bidId: number;
  days?: number;
}

export const AuditAssistant: React.FC<AuditAssistantProps> = ({
  projectId,
  bidId,
}) => {
  const aiChat = useAiChat({ projectId, bidId });

  return (
    <div style={{ height: '100%' }}>
      <ChatWindow
        messages={aiChat.messages}
        isLoading={aiChat.isLoading}
        isHistoryLoading={aiChat.isHistoryLoading}
        onSend={aiChat.sendMessage}
        onClear={aiChat.clearMessages}
      />
    </div>
  );
};
