import { useEffect } from 'react';
import { Button, Modal, Form, Input, Select } from 'antd';
import { useStyles } from '../style';
import {
   CategoryMap,
   ApplicableScopeMap,
   type KnowledgeFile,
   type ApplicableScope,
   type KnowledgeFileCategory,
} from '../types';
import { useIsMobile } from '@/hooks/useMediaQuery';

// 定义表单的数据结构
export interface EditFormValues {
   fileName: string;
   category: KnowledgeFileCategory;
   applicableScope: ApplicableScope;
   description?: string;
   status: 0 | 1;
}

// --- 编辑弹窗组件 Props ---
interface EditModalProps {
   visible: boolean;
   selectedFile: KnowledgeFile | null;
   onCancel: () => void;
   onConfirm: (values: EditFormValues) => Promise<void>;
   submitting?: boolean;
}

// --- 编辑弹窗组件 ---
export function EditModal({
   visible,
   selectedFile,
   onCancel,
   onConfirm,
   submitting = false,
}: EditModalProps) {
   const { styles } = useStyles();
   const isMobile = useIsMobile();
   const [editForm] = Form.useForm<EditFormValues>();

   const normalizeStatus = (status: unknown): 0 | 1 => {
      const value = String(status ?? '');
      if (value === '1' || value.toLowerCase() === 'enabled' || value === '启用') {
         return 1;
      }
      return 0;
   };

   // --- 初始化与重置表单数据 ---
   useEffect(() => {
      if (visible && selectedFile) {
         editForm.resetFields();
         editForm.setFieldsValue({
            fileName: selectedFile.fileName || '',
            category: selectedFile.category || '',
            applicableScope: selectedFile.applicableScope || 'general',
            description: selectedFile.description || '',
            status: normalizeStatus(selectedFile.status),
         });
      }
   }, [visible, selectedFile, editForm]);

   // --- 提交编辑 ---
   const handleSubmit = async () => {
      try {
         const values = await editForm.validateFields();
         await onConfirm(values);
      } catch (error) {
         console.error('Edit validation failed:', error);
      }
   };

   const handleCancel = () => {
      onCancel();
      setTimeout(() => {
         editForm.resetFields();
      }, 300);
   };

   return (
      <Modal
         title='编辑文件信息'
         open={visible}
         onCancel={handleCancel}
         maskClosable={!submitting}
         closable={!submitting}
         width={600}
         centered={true}
         footer={[
            <Button key='cancel' onClick={handleCancel} disabled={submitting}>
               取消
            </Button>,
            <Button
               key='submit'
               type='primary'
               onClick={handleSubmit}
               className={styles.uploadBtn}
               loading={submitting}
            >
               确认修改
            </Button>,
         ]}
      >
         <Form
            form={editForm}
            layout='vertical'
            style={{
               height: isMobile ? '50vh' : '60vh',
               overflowY: 'auto',
               padding: '0 1rem',
               scrollbarWidth: 'none',
            }}
         >
            <Form.Item
               label='文件名称'
               name='fileName'
               rules={[{ required: true, message: '请输入文件名称' }]}
            >
               <Input placeholder='请输入文件名称' />
            </Form.Item>

            <Form.Item
               label='文件分类'
               name='category'
               rules={[{ required: true, message: '请选择文件分类' }]}
            >
               <Select
                  placeholder='请选择文件分类'
                  options={Object.entries(CategoryMap).map(([key, label]) => ({
                     value: key,
                     label: label,
                  }))}
               />
            </Form.Item>

            <Form.Item
               label='适用范围'
               name='applicableScope'
               rules={[{ required: true, message: '请选择适用范围' }]}
            >
               <Select
                  placeholder='请选择适用范围'
                  options={Object.entries(ApplicableScopeMap).map(
                     ([key, label]) => ({
                        value: key,
                        label: label,
                     })
                  )}
               />
            </Form.Item>

            <Form.Item label='用途描述' name='description'>
               <Input.TextArea placeholder='请输入用途描述（选填）' rows={3} />
            </Form.Item>

            <Form.Item label='状态' name='status'>
               <Select
                  placeholder='请选择状态'
                  options={[
                     { value: 1, label: '启用' },
                     { value: 0, label: '停用' },
                  ]}
               />
            </Form.Item>
         </Form>
      </Modal>
   );
}
