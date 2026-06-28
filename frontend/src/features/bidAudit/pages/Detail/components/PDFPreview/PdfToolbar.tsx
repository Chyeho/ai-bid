import React, { useRef } from 'react';
import { Space, Button, Tooltip, Typography, InputNumber } from 'antd';
import { ZoomInOutlined, ZoomOutOutlined } from '@ant-design/icons';
import { useStyles } from '../../style';

const { Text } = Typography;

interface PdfToolbarProps {
   scale: number;
   currentPage: number;
   numPages: number;
   onZoomIn: () => void;
   onZoomOut: () => void;
   onResetZoom: () => void;
   onJumpToPage: (page: number) => void;
}

export const PdfToolbar: React.FC<PdfToolbarProps> = React.memo((props) => {
   const { styles } = useStyles();
   const pendingPageRef = useRef<number | null>(null);

   const handleJump = () => {
      const value =
         pendingPageRef.current == null
            ? props.currentPage
            : pendingPageRef.current;
      if (value < 1 || value > props.numPages) return;
      props.onJumpToPage(value);
   };

   return (
      <div className={styles.toolbar}>
         <Space size='large'>
            <Space.Compact>
               <Tooltip title='回车键(Enter)进行跳转'>
                  <InputNumber
                     min={1}
                     max={props.numPages}
                     defaultValue={props.currentPage}
                     onChange={(val) => {
                        pendingPageRef.current =
                           typeof val === 'number' ? val : null;
                     }}
                     onPressEnter={handleJump}
                     onBlur={handleJump}
                     style={{ width: 60 }}
                     controls={false}
                  />
               </Tooltip>
            </Space.Compact>

            <Text type='secondary' style={{ color: '#5a5a5a' }}>
               {props.currentPage} / {props.numPages} 页
            </Text>
         </Space>

         <Space.Compact>
            <Tooltip title='最小10%'>
               <Button
                  className={styles.actionBtn}
                  icon={<ZoomOutOutlined />}
                  onClick={props.onZoomOut}
               />
            </Tooltip>

            <Tooltip title='单击返回 60% 比例'>
               <Button
                  className={styles.actionBtn}
                  onClick={props.onResetZoom}
                  style={{ width: 65 }}
               >
                  {Math.round(props.scale * 100)}%
               </Button>
            </Tooltip>

            <Tooltip title='最大100%'>
               <Button
                  className={styles.actionBtn}
                  icon={<ZoomInOutlined />}
                  onClick={props.onZoomIn}
               />
            </Tooltip>
         </Space.Compact>
      </div>
   );
});
