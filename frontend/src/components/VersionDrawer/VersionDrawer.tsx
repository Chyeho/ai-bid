import { useRef, useState } from 'react';

import {
   Drawer,
   Timeline,
   Card,
   Descriptions,
   Button,
   List,
   Tag,
   type DescriptionsProps,
} from 'antd';

import { useNavigate } from 'react-router-dom';

import type { ParseStatusType, ProjectItem } from '@/features/bidAudit/types';
import { CloseOutlined } from '@ant-design/icons';
import { useIsMobile } from '@/hooks/useMediaQuery';
import { EllipsisTooltip } from '@/components/EllipsisTooltip/EllipsisTooltip';
import dayjs from 'dayjs';

interface VersionDrawerProps {
   open: boolean;
   onClose: () => void;
   versions: ProjectItem[];
   isFetching: boolean;
}

const parseStatusMap: Record<ParseStatusType, { text: string; color: string }> = {
   0: { text: '待审核', color: 'blue' },
   1: { text: '审核中', color: 'orange' },
   2: { text: '审核完成', color: 'green' },
   3: { text: '审核失败', color: 'red' },
};

export const VersionDrawer = ({
   open,
   onClose,
   versions,
   isFetching,
}: VersionDrawerProps) => {
   const cardRefs = useRef<(HTMLDivElement | null)[]>([]);
   const navigate = useNavigate();
   const isMobile = useIsMobile();

   const [drawerWidth] = useState<string | number>('72vw');

   const handleJump = (index: number) => {
      cardRefs.current[index]?.scrollIntoView({
         behavior: 'smooth',
         block: 'start',
      });
   };

   const parseStatusTag = (
      parseStatus: ParseStatusType,
      auditResult?: string | null
   ) => {
      if (parseStatus === 2) {
         if (auditResult === 'pass') {
            return (
               <Tag color='green'>
                  审核完成-通过
               </Tag>
            );
         }
         return (
            <Tag color='gold'>
               审核完成-需修改
            </Tag>
         );
      }
      const config = parseStatusMap[parseStatus] || {
         text: '未知',
         color: 'default',
      };

      return (
         <Tag color={config.color}>
            {config.text}
         </Tag>
      );
   };

   const getDescriptionsItems = (
      item: ProjectItem
   ): DescriptionsProps['items'] => [
      {
         key: 'bidName',
         label: '文件名',
         children: <EllipsisTooltip text={item.bidName || '-'} />,
         span: isMobile ? 1 : 2,
      },
      { key: 'fileSize', label: '文件大小', children: `${item.fileSize}B` },
      {
         key: 'uploadTime',
         label: '上传时间',
         children: item.uploadTime
            ? dayjs(item.uploadTime).format('YYYY-MM-DD')
            : '-',
      },
      { key: 'pageCount', label: '页数', children: item.pageCount },
      {
         key: 'parseStatus',
         label: '审核状态',
         children: parseStatusTag(item.parseStatus, item.auditResult),
      },
      { key: 'auditorName', label: '审核人', children: item.auditorName },
   ];

   return (
      <Drawer
         title={'项目历史版本'}
         open={open}
         onClose={onClose}
         placement={isMobile ? 'top' : 'right'}
         closeIcon={
            <CloseOutlined
               style={{
                  fontSize: '2rem',
                  color: 'green',
               }}
            />
         }
         width={isMobile ? '100%' : drawerWidth}
         height={isMobile ? '70vh' : '100%'}
         loading={isFetching}
         styles={{
            body: { padding: '1rem 1.5rem', scrollbarWidth: 'none' },
            header: { padding: '1.5rem' },
         }}
      >
         <div style={{ display: 'flex', gap: isMobile ? '0.5rem' : '0.75rem' }}>
            <div style={{ minWidth: isMobile ? 70 : 80, flexShrink: 0 }}>
               <div style={{ position: 'sticky', top: 0 }}>
                  <List
                     header={<strong>版本目录</strong>}
                     dataSource={versions}
                     renderItem={(item, index) => (
                        <List.Item style={{ padding: '2px 0', border: 'none' }}>
                           <Button
                              type='link'
                              onClick={() => handleJump(index)}
                           >
                              V{item.version}
                           </Button>
                        </List.Item>
                     )}
                     style={{
                        display: 'flex',
                        flexDirection: 'column',
                        alignItems: 'center',
                     }}
                  />
               </div>
            </div>

            <div style={{ flex: 1 }}>
               <Timeline
                  items={versions.map((item, index) => ({
                     key: item.id,
                     children: (
                        <div
                           ref={(el) => {
                              cardRefs.current[index] = el;
                           }}
                        >
                           <Card
                              title={`V${item.version}`}
                              extra={
                                 <Button
                                    style={{ fontSize: '1.2rem' }}
                                    onClick={() =>
                                       navigate(`/bidReview/detail/${item.id}`)
                                    }
                                 >
                                    {index === 0 && item.parseStatus === 0
                                       ? '进入审核'
                                       : '查看审核详情'}
                                 </Button>
                              }
                              styles={{
                                 header: { padding: '1rem 1.25rem' },
                                 body: {
                                    padding: '0.9rem 1.1rem',
                                 },
                              }}
                              style={{ marginBottom: '0.75rem' }}
                           >
                              <Descriptions
                                 size='small'
                                 column={isMobile ? 1 : 3}
                                 items={getDescriptionsItems(item)}
                                 styles={{ label: { fontSize: '1.1rem' } }}
                              />
                           </Card>
                        </div>
                     ),
                  }))}
               ></Timeline>
            </div>
         </div>
      </Drawer>
   );
};
