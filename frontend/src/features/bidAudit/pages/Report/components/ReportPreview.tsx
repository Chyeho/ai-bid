import { forwardRef } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { Typography } from 'antd';
import { useStyles } from '../style';

const { Text } = Typography;

interface ReportPreviewProps {
   scale: number;
   markdownContent: string;
}

export const ReportPreview = forwardRef<HTMLDivElement, ReportPreviewProps>(
   ({ scale, markdownContent }, ref) => {
      const { styles } = useStyles();

      return (
         <div className={styles.previewArea} ref={ref}>
            <div
               className={styles.a4Paper}
               style={{ transform: `scale(${scale / 100})` }}
            >
               {markdownContent ? (
                  <ReactMarkdown remarkPlugins={[remarkGfm]}>
                     {markdownContent}
                  </ReactMarkdown>
               ) : (
                  <div
                     style={{
                        display: 'flex',
                        justifyContent: 'center',
                        alignItems: 'center',
                        height: '100%',
                        color: '#999',
                     }}
                  >
                     <Text type='secondary'>
                        暂无报告内容，请勾选左侧报告模块
                     </Text>
                  </div>
               )}
            </div>
         </div>
      );
   }
);

ReportPreview.displayName = 'ReportPreview';
