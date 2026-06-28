import React, { useState } from 'react';
import { Input, Button, Tooltip } from 'antd';
import { SendOutlined, ClearOutlined, SaveOutlined, ExclamationCircleOutlined } from '@ant-design/icons';

type ChatInputMode = 'default' | 'supplement';

interface ChatInputProps {
   onSend: (content: string, mode: ChatInputMode) => void;
   onClear: () => void;
   onSave: () => void;
   isSaving: boolean;
   disabled: boolean;
   hasSavableSupplement: boolean;
   savableSupplementCount: number;
}

export const ChatInput: React.FC<ChatInputProps> = ({
   onSend,
   onClear,
   onSave,
   isSaving,
   disabled,
   hasSavableSupplement,
   savableSupplementCount,
}) => {
   const [val, setVal] = useState('');
   const [mode, setMode] = useState<ChatInputMode>('default');

   const handleSend = () => {
      if (!val.trim()) return;
      onSend(val, mode);
      setVal('');
      setMode('default');
   };

   const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
         e.preventDefault();
         handleSend();
      }
   };

   return (
      <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
         <div style={{ display: 'flex', gap: 6, alignItems: 'center', justifyContent: 'flex-end' }}>
            <Tooltip title='清除对话'>
               <Button type='text' icon={<ClearOutlined />} onClick={onClear} />
            </Tooltip>
            <Tooltip title='补充错误模式'>
               <Button
                  type={mode === 'supplement' ? 'primary' : 'default'}
                  size='small'
                  icon={<ExclamationCircleOutlined />}
                  onClick={() =>
                     setMode((prev) => (prev === 'supplement' ? 'default' : 'supplement'))
                  }
                  disabled={disabled}
               >
                  补充错误
               </Button>
            </Tooltip>
            <Tooltip
               title={
                  hasSavableSupplement
                     ? `保存 ${savableSupplementCount} 条补充错误`
                     : '暂无补充错误可保存'
               }
            >
               <Button
                  type='primary'
                  size='small'
                  icon={<SaveOutlined />}
                  onClick={onSave}
                  loading={isSaving}
                  disabled={isSaving}
               >
                  保存补充{hasSavableSupplement ? ` (${savableSupplementCount})` : ''}
               </Button>
            </Tooltip>
         </div>

         {mode === 'supplement' && (
            <div style={{ color: '#000', fontSize: 13, fontWeight: 500 }}>
               补充错误：
            </div>
         )}

         <div style={{ display: 'flex', gap: 8 }}>
            <Input.TextArea
               value={val}
               onChange={(e) => setVal(e.target.value)}
               onKeyDown={handleKeyDown}
               disabled={disabled}
               autoSize={{ minRows: 1, maxRows: 3 }}
               style={{ scrollbarWidth: 'none' }}
               placeholder={
                  mode === 'supplement'
                     ? '请描述你要补充/纠正的错误，Ctrl + Enter 发送'
                     : 'Ctrl + Enter(回车键) 发送'
               }
            />

            <Button
               type='primary'
               icon={<SendOutlined />}
               onClick={handleSend}
               disabled={!val.trim() || disabled}
            />
         </div>
      </div>
   );
};
