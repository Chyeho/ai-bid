import React, { useState } from 'react';
import { SendOutlined } from '@ant-design/icons';
import { useStyles } from './style';

interface ChatInputProps {
  onSend: (content: string) => void;
  disabled: boolean;
}

export const ChatInput: React.FC<ChatInputProps> = ({ onSend, disabled }) => {
  const { styles } = useStyles();
  const [val, setVal] = useState('');

  const handleSend = () => {
    if (!val.trim()) return;
    onSend(val.trim());
    setVal('');
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  return (
    <div className={styles.inputRow}>
      <textarea
        className={styles.textArea}
        value={val}
        onChange={(e) => setVal(e.target.value)}
        onKeyDown={handleKeyDown}
        disabled={disabled}
        rows={1}
        placeholder="Enter 发送，Shift+Enter 换行"
        style={{ scrollbarWidth: 'none' }}
      />
      <button
        className={styles.sendBtn}
        onClick={handleSend}
        disabled={!val.trim() || disabled}
        type="button"
      >
        <SendOutlined />
      </button>
    </div>
  );
};
