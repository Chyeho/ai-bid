import React, { useRef, useEffect, useCallback } from 'react';
import { Spin, Typography } from 'antd';
import { useStyles } from './style';
import { MessageBubble } from './MessageBubble';
import { ChatInput } from './ChatInput';
import type { ChatMessage, ChatMode } from '../../hooks/useAiChat';

const { Text } = Typography;

interface ChatWindowProps {
   messages: ChatMessage[];
   isLoading: boolean;
   isHistoryLoading?: boolean;
   onSend: (content: string, mode?: ChatMode) => void;
   onClear: () => void;
   onSave?: () => void;
   isSaving?: boolean;
   hasSavableSupplement?: boolean;
   savableSupplementCount?: number;
}

export const ChatWindow: React.FC<ChatWindowProps> = ({
   messages,
   isLoading,
   isHistoryLoading,
   onSend,
   onClear,
   onSave,
   isSaving,
   hasSavableSupplement,
   savableSupplementCount,
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
      (content: string, mode?: ChatMode) => {
         hasInteracted.current = true;
         onSend(content, mode);
      },
      [onSend]
   );

   const handleClear = useCallback(() => {
      hasInteracted.current = false;
      onClear();
   }, [onClear]);

   return (
      <div className={styles.container}>
         <div
            className={styles.messageList}
         >
            {isHistoryLoading ? (
               <div className={styles.centered}>
                  <Spin />
               </div>
            ) : messages.length === 0 ? (
               <div className={styles.centered}>
                  <Text type='secondary'>随意提问</Text>
               </div>
            ) : (
               messages.map((m: ChatMessage) => (
                  <MessageBubble key={m.id} message={m} />
               ))
            )}
            {isLoading && (
               <div className={styles.typing}>
                  <Spin size='small' />{' '}
                  <Text type='secondary' style={{ marginLeft: 8 }}>
                     分析中...
                  </Text>
               </div>
            )}
            <div ref={bottomRef} />
         </div>

         <footer className={styles.footer}>
            <ChatInput
               onSend={handleSend}
               onClear={handleClear}
               onSave={onSave ?? (() => {})}
               isSaving={isSaving ?? false}
               disabled={isLoading}
               hasSavableSupplement={hasSavableSupplement ?? false}
               savableSupplementCount={savableSupplementCount ?? 0}
            />
         </footer>
      </div>
   );
};
