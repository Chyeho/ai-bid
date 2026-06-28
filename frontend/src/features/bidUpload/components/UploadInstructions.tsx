import React from 'react';
import { Card, Button, Typography } from 'antd';
import { UpOutlined, DownOutlined } from '@ant-design/icons';
import { useStyles } from '../style';

const { Text } = Typography;

interface UploadInstructionsProps {
   collapsed: boolean;
   setCollapsed: (value: boolean) => void;
   isMobile: boolean;
}

export const UploadInstructions: React.FC<UploadInstructionsProps> = ({
   collapsed,
   setCollapsed,
   isMobile,
}) => {
   const { theme } = useStyles();

   return (
      <Card
         title={
            <div
               style={{
                  display: 'flex',
                  justifyContent: 'space-between',
                  alignItems: 'center',
               }}
            >
               <span
                  style={{
                     fontWeight: 600,
                     fontSize: 16,
                     color: theme.colorTextBase,
                  }}
               >
                  上传须知
               </span>
               <Button
                  type='text'
                  icon={collapsed ? <UpOutlined /> : <DownOutlined />}
                  onClick={() => setCollapsed(!collapsed)}
               />
            </div>
         }
         style={{
            borderRadius: theme.borderRadiusLG,
            boxShadow: theme.boxShadow,
            background: theme.colorBgContainer,
            width: isMobile ? '100%' : 300,
         }}
         styles={{ header: { borderBottom: `1px solid ${theme.colorBorder}` } }}
      >
         {!collapsed && (
            <Text
               style={{
                  lineHeight: 1.8,
                  color: theme.colorTextDescription,
                  fontSize: 14,
               }}
            >
               • 支持 Word (.doc, .docx) 和 PDF (.pdf) 格式，单文件不超过50MB
               <br />•
               标书应在检查过、纳税后、商务后、供应商检查后上传整套标书流程，并对通过开始审核。
            </Text>
         )}
      </Card>
   );
};
