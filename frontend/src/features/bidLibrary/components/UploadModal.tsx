import { useState, useRef } from 'react';
import { Button, Modal, Form, Upload, Input, Select, message } from 'antd';
import { UploadOutlined } from '@ant-design/icons';
import type { UploadFile } from 'antd/es/upload/interface';
import { useStyles } from '../style';
import { CategoryMap } from '../types';
import { useIsMobile } from '@/hooks/useMediaQuery';

interface UploadModalProps {
   visible: boolean; // 弹窗显示状态
   onCancel: () => void; // 取消回调
   onConfirm: (formData: FormData) => void; // 确认上传回调，传入组装好的 FormData
}

export function UploadModal({
   visible,
   onCancel,
   onConfirm,
}: UploadModalProps) {
   const { styles } = useStyles();
   const isMobile = useIsMobile();
   const [uploadForm] = Form.useForm();
   const [fileList, setFileList] = useState<UploadFile[]>([]);
   const fileNameModifiedRef = useRef(false);

   const handleFileChange = ({ fileList: newFileList }: { fileList: UploadFile[] }) => {
      setFileList(newFileList);

      // 当有新文件添加时，自动填充文件名（仅当文件名未被手动修改过时）
      if (newFileList.length > 0 && !fileNameModifiedRef.current) {
         const file = newFileList[0];
         const fileName = file.name;
         uploadForm.setFieldValue('fileName', fileName);
      }
   };

   const handleFileNameChange = () => {
      fileNameModifiedRef.current = true;
   };

   const handleCancel = () => {
      fileNameModifiedRef.current = false;
      setFileList([]);
      uploadForm.resetFields();
      onCancel();
   };

   const handleConfirm = () => {
      uploadForm.validateFields().then((values) => {
         if (fileList.length === 0) {
            message.error('请先选择文件');
            return;
         }
         const rawFile = fileList[0]?.originFileObj;
         if (!rawFile) {
            message.error('文件对象无效，请重新选择文件');
            return;
         }

         const formData = new FormData();
         formData.append('file', rawFile);
         formData.append('fileName', values.fileName || fileList[0].name); // 如果没填，用文件本身的名字
         formData.append('category', values.category);
         formData.append('applicableScope', values.applicableScope);
         formData.append('description', values.description || '');
         formData.append('status', String(values.status ?? 1));

         console.log('正在发送的 FormData:', Object.fromEntries(formData));

         onConfirm(formData);
         uploadForm.resetFields();
         setFileList([]);
         fileNameModifiedRef.current = false;
      }).catch(err => {
         console.error('表单校验失败:', err);
      });
   };

   return (
      <Modal
         title='上传标准库文件'
         open={visible}
         centered={true}
         onCancel={handleCancel}
         width={600}
         footer={[
            <Button key='cancel' onClick={handleCancel}>
               取消
            </Button>,
            <Button
               key='submit'
               type='primary'
               onClick={handleConfirm}
               className={styles.uploadBtn}
            >
               确认上传
            </Button>,
         ]}
      >
         <Form
            form={uploadForm}
            initialValues={{ status: 1 }}
            layout='vertical'
            style={{
               height: isMobile ? '50vh' : '60vh',
               overflowY: 'auto',
               padding: '0 1rem',
               scrollbarWidth: 'none',
            }}
         >
            <Form.Item label='选择文件'>
               <Upload
                  beforeUpload={() => false}
                  maxCount={1}
                  fileList={fileList}
                  onChange={handleFileChange}
               >
                  <Button icon={<UploadOutlined />}>点击上传</Button>
               </Upload>
            </Form.Item>

            <Form.Item
               label='文件名称'
               name='fileName'
               rules={[{ required: true, message: '请输入文件名称' }]}
            >
               <Input
                  placeholder='请输入文件名称'
                  onChange={handleFileNameChange}
               />
            </Form.Item>

            <Form.Item
               label='文件类型'
               name='category'
               rules={[{ required: true, message: '请选择文件类型' }]}
            >
               <Select
                  placeholder='请选择文件类型'
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
                  options={[
                     { value: 'procurement', label: '采购类' },
                     { value: 'engineering', label: '工程类' },
                     { value: 'general', label: '通用' },
                  ]}
               />
            </Form.Item>

            <Form.Item
               label='状态'
               name='status'
               rules={[{ required: true, message: '请选择状态' }]}
            >
               <Select
                  options={[
                     { value: 1, label: '启用' },
                     { value: 0, label: '停用' },
                  ]}
               />
            </Form.Item>

            <Form.Item label='用途描述' name='description'>
               <Input.TextArea placeholder='请输入用途描述（选填）' rows={3} />
            </Form.Item>
         </Form>
      </Modal>
   );
}
