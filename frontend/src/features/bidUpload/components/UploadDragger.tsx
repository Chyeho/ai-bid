import React from 'react';
import { Upload, Button, Typography, App } from 'antd';
import {
   CloudUploadOutlined,
   FileTextOutlined,
   CloseOutlined,
} from '@ant-design/icons';
import { useStyles } from '../style';

const { Text } = Typography;

interface Props {
   file: File | null;
   onFileChange: (file: File | null) => void;
}

export const UploadDragger: React.FC<Props> = React.memo(
   ({ file, onFileChange }) => {
      const { styles, theme } = useStyles();
      const { message } = App.useApp();
      
      const beforeUpload = (f: File) => {
         const isValid = [
            'application/pdf',
            'application/msword',
            'application/vnd.openxmlformats-officedocument.wordprocessingml.document',
         ].includes(f.type);
         if (!isValid) {
            message.error('仅支持 Word/PDF！');
            return Upload.LIST_IGNORE;
         }
         if (f.size / 1024 / 1024 > 50) {
            message.error('文件过大！');
            return Upload.LIST_IGNORE;
         }
         onFileChange(f);
         return false;
      };

      return (
         <>
            <Upload
               beforeUpload={beforeUpload}
               showUploadList={false}
               maxCount={1}
               style={{
                  width: '100%',
                  maxWidth: '800px',
                  marginBottom: '2rem',
               }}
            >
               <div className={styles.uploadDragger}>
                  <CloudUploadOutlined
                     style={{ fontSize: 32, color: theme.colorPrimary }}
                  />
                  <p style={{ margin: '8px 0', fontWeight: 600 }}>
                     拖拽文件到此处(或点击选择文件)
                  </p>
                  <Text type='secondary' style={{ fontSize: 12 }}>
                     仅支持单 Word/PDF 文件类型，不超过 50MB
                  </Text>
               </div>
            </Upload>

            {file && (
               <div className={styles.fileListItem}>
                  <Text>
                     <FileTextOutlined style={{ color: theme.colorPrimary }} />{' '}
                     {file.name}
                  </Text>

                  <Button
                     type='text'
                     icon={<CloseOutlined />}
                     onClick={() => onFileChange(null)}
                  />
               </div>
            )}
         </>
      );
   }
);
