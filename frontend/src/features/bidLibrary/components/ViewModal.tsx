import { Button, Modal, Tag } from 'antd';
import { useStyles } from '../style';
import { CategoryMap, type KnowledgeFile } from '../types';

interface ViewModalProps {
   visible: boolean; // 弹窗显示状态
   selectedFile: KnowledgeFile | null; // 选中的要查看的文件
   onCancel: () => void; // 关闭弹窗回调
}

// --- 查看弹窗组件 ---
export function ViewModal({ visible, selectedFile, onCancel }: ViewModalProps) {
   const { theme } = useStyles();

   return (
      <Modal
         title='文件详情'
         open={visible}
         onCancel={onCancel}
         footer={[
            <Button key='close' onClick={onCancel}>
               关闭
            </Button>,
         ]}
      >
         {selectedFile && (
            <div>
               <p>
                  <strong>文件名称：</strong>
                  {selectedFile.fileName}
               </p>
               <p>
                  <strong>文件类型：</strong>
                  <Tag
                     style={{
                        backgroundColor: theme.colorSuccessBg || '#f6ffed',
                        color: theme.colorSuccess || '#52c41a',
                        border: 'none',
                     }}
                  >
                     {
                        CategoryMap[
                           selectedFile.category as keyof typeof CategoryMap
                        ]
                     }
                  </Tag>
               </p>
               <p>
                  <strong>文件大小：</strong>
                  {(selectedFile.fileSize / 1024).toFixed(2)} KB
               </p>
               <p>
                  <strong>上传人：</strong>
                  {selectedFile.uploadUserName}
               </p>
               <p>
                  <strong>上传时间：</strong>
                  {selectedFile.uploadTime}
               </p>
               <p>
                  <strong>版本：</strong>
                  {selectedFile.version}
               </p>
               <p>
                  <strong>状态：</strong>
                  <Tag
                     style={{
                        backgroundColor:
                           selectedFile.status === 1
                              ? theme.colorSuccessBg
                              : theme.colorBgContainerDisabled,
                        color:
                           selectedFile.status === 1
                              ? theme.colorSuccess
                              : theme.colorTextSecondary,
                        border: 'none',
                     }}
                  >
                     {selectedFile.status === 1 ? '启用' : '停用'}
                  </Tag>
               </p>
               <p>
                  <strong>描述：</strong>
                  {selectedFile.description || '暂无描述'}
               </p>
            </div>
         )}
      </Modal>
   );
}
