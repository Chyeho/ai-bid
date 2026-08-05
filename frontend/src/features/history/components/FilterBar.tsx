import React, { useEffect } from 'react';
import { useStyles } from '../style';
import { useDebounce } from '@/hooks/useDebounce';
import { useIsMobile } from '@/hooks/useMediaQuery';
import type { HistoryQueryParams } from '../types';

import { Form, Input, Select, Button, Row, Col, Space } from 'antd';
import { SearchOutlined, ReloadOutlined } from '@ant-design/icons';
import dayjs from 'dayjs';
import { ResponsiveRangePicker } from '@/components/ResponsiveRangePicker/ResponsiveRangePicker';

interface FilterBarProps {
   initialValues: Partial<HistoryQueryParams>;
   onSearch: (values: Partial<HistoryQueryParams>) => void;
   onReset: () => void;
}

const FILE_TYPE_OPTIONS = [
   { label: '全部', value: '' },
   { label: '标书', value: '标书' },
   { label: '合同', value: '合同' },
];

export const FilterBar: React.FC<FilterBarProps> = ({
   initialValues,
   onSearch,
   onReset,
}) => {
   const [form] = Form.useForm();
   const { styles } = useStyles();
   const isMobile = useIsMobile();
   const { run: debouncedSubmit } = useDebounce(() => form.submit(), 500);

   useEffect(() => {
      const { startDate, endDate, ...rest } = initialValues;
      const formValues: any = { ...rest };

      if (startDate && endDate) {
         formValues.dateRange = [dayjs(startDate), dayjs(endDate)];
      } else {
         formValues.dateRange = undefined;
      }
      form.setFieldsValue(formValues);
   }, [initialValues, form]);

   const handleFinish = (values: any) => {
      const { dateRange, ...rest } = values;
      let startDate = '';
      let endDate = '';

      if (dateRange && dateRange.length === 2) {
         startDate = dateRange[0].format('YYYY-MM-DD');
         endDate = dateRange[1].format('YYYY-MM-DD');
      }

      onSearch({
         ...rest,
         startDate,
         endDate,
      });
   };

   const handleReset = () => {
      form.resetFields();
      onReset();
   };

   return (
      <div className={styles.cardWrapper}>
         <Form
            form={form}
            name='history_filter'
            onFinish={handleFinish}
            className={styles.filterForm}
         >
            <Row
               gutter={16}
               style={{
                  display: 'flex',
                  alignItems: 'center',
               }}
            >
               <Col
                  sm={12}
                  md={8}
                  flex={1}
                  style={{
                     width: '100%',
                     maxWidth: isMobile ? '100%' : '350px',
                  }}
               >
                  <Form.Item name='projectName' label='项目名称'>
                     <Input
                        placeholder='请输入项目名称'
                        allowClear
                        onChange={debouncedSubmit}
                        style={{ height: 30 }}
                     />
                  </Form.Item>
               </Col>

               <Col sm={12} md={8} flex={isMobile ? 1 : 'none'}>
                  <Form.Item name='dateRange' label='审核时间'>
                     <ResponsiveRangePicker />
                  </Form.Item>
               </Col>
            </Row>

            <Row gutter={16}>
               <Col xs={24} sm={8} md={6}>
                  <Form.Item name='fileCategory' label='文件类型'>
                     <Select
                        options={FILE_TYPE_OPTIONS}
                        placeholder='请选择'
                        allowClear
                     />
                  </Form.Item>
               </Col>

               <Col xs={24} sm={8} md={6}>
                  <Form.Item name='auditUserName' label='审核人'>
                     <Input
                        placeholder='请搜索'
                        onChange={debouncedSubmit}
                        style={{ height: 35 }}
                        allowClear
                     />
                  </Form.Item>
               </Col>

               <Col xs={24} sm={24} md={6}>
                  <Form.Item label={isMobile ? null : ' '} colon={false}>
                     <Space
                        style={{
                           display: 'flex',
                           justifyContent: isMobile ? 'flex-end' : 'flex-start',
                        }}
                     >
                        <Button
                           type='primary'
                           htmlType='submit'
                           icon={<SearchOutlined />}
                           style={{ fontSize: '1.2rem' }}
                        >
                           查询
                        </Button>

                        <Button
                           icon={<ReloadOutlined />}
                           onClick={handleReset}
                           style={{ fontSize: '1.2rem' }}
                        >
                           重置
                        </Button>
                     </Space>
                  </Form.Item>
               </Col>
            </Row>
         </Form>
      </div>
   );
};
