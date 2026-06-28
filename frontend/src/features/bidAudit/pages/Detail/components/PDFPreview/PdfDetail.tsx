import React from 'react';
import { Row, Col, Typography } from 'antd';
import { useStyles } from '../../style';
import { useParams } from 'react-router-dom';
import { auditDetailOptions } from '../../api/auditDetail';
import { useQuery } from '@tanstack/react-query';
import { Loading } from '@/components/Loading/Loading';

const { Text } = Typography;

export const PdfDetail: React.FC = () => {
   const { styles } = useStyles();

   const { id: bidId } = useParams<{ id: string }>();

   const { data, isLoading, isError } = useQuery({
      ...auditDetailOptions.bidDetail(Number(bidId)),
      enabled: !!bidId && !isNaN(Number(bidId)),
   });

   if (isLoading) {
      return <Loading loading={isLoading} />;
   }

   if (isError || !data) {
      return (
         <div className={styles.detailContainer}>
            <Text type='secondary'>暂无项目详细信息或加载失败</Text>
         </div>
      );
   }

   return (
      <div className={styles.detailContainer}>
         <Row gutter={[16, 12]}>
            <Col span={24}>
               <span className={styles.label}>项目名称：</span>
               <span
                  className={styles.value}
                  style={{ fontSize: 16, fontWeight: 600 }}
               >
                  {data.bidName}
               </span>
            </Col>

            <Col span={12}>
               <span className={styles.label}>预算金额：</span>
               <span className={styles.value}>{data.budgetAmount}</span>
            </Col>

            <Col span={12}>
               <span className={styles.label}>标书类型：</span>
               <span className={styles.value}>{data.fileCategory}</span>
            </Col>

            <Col span={12}>
               <span className={styles.label}>上传时间：</span>
               <span className={styles.value}>{data.uploadTime}</span>
            </Col>

            <Col span={12}>
               <span className={styles.label}>供应商名称：</span>
               <span className={styles.value}>{data.supplierName}</span>
            </Col>
         </Row>
      </div>
   );
};
