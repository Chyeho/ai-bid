import type { ProjectItem, BidDetail } from '../types';

const mockProjectNames = [
   '校园网络改造项目',
   '图书馆设备采购项目',
   '实验室通风系统项目',
   '宿舍楼维修项目',
   '教学楼多媒体设备项目',
   '食堂设备更新项目',
   '体育馆照明系统项目',
   '办公楼空调系统项目',
   '实训基地建设项目',
   '智慧教室建设项目',
];

const mockSuppliers = [
   '中科信息有限公司',
   '神州数码有限公司',
   '建工集团有限公司',
   '三建工程有限公司',
   '创新科技有限公司',
   '华信电子有限公司',
   '远大建筑有限公司',
   '盛世科技发展有限公司',
   '联创智能有限公司',
   '恒达设备有限公司',
];

const generateMockData = (): ProjectItem[] => {
   return Array.from({ length: 10 }, (_, index) => {
      const parseStatus = Math.random() > 0.5 ? 1 : 0;
      const createTime = new Date();
      createTime.setDate(createTime.getDate() - Math.floor(Math.random() * 30));

      const bidDetail: BidDetail = {
         tender: {
            id: index + 1,
            fileName: `${mockProjectNames[index]}-招标文件.pdf`,
            filePath: `/files/bid_${index + 1}.pdf`,
            fileSize: Math.floor(Math.random() * 10000000) + 1000000,
            fileType: Math.random() > 0.5 ? 'pdf' : 'word',
            fileCategory: Math.random() > 0.5 ? 'bid' : 'contract',
            bidName: mockProjectNames[index],
            supplierName: mockSuppliers[index],
            budgetAmount: Math.floor(Math.random() * 1000000) + 100000,
            pageCount: Math.floor(Math.random() * 200) + 50,
            parseStatus: parseStatus,
            uploadUserId: Math.floor(Math.random() * 10) + 1,
            uploadTime: createTime.toISOString(),
            version: Math.floor(Math.random() * 5) + 1,
            projectId: index + 1,
         },
         auditTask: {
            id: index + 1,
            taskId: `TASK-${String(index + 1).padStart(4, '0')}`,
            bidId: index + 1,
            taskStatus: parseStatus === 1 ? 2 : 1,
            auditResult: parseStatus === 1 ? '通过' : '待审核',
            issueCount: parseStatus === 1 ? Math.floor(Math.random() * 5) : 0,
            criticalCount: Math.floor(Math.random() * 3),
            warningCount: Math.floor(Math.random() * 5),
            infoCount: Math.floor(Math.random() * 10),
            startTime: createTime.toISOString(),
            endTime: parseStatus === 1 ? createTime.toISOString() : '',
            auditUserId: Math.floor(Math.random() * 10) + 1,
            createTime: createTime.toISOString(),
         },
         auditReport: {
            id: index + 1,
            auditId: index + 1,
            docContent: '审核报告内容...',
            version: Math.floor(Math.random() * 5) + 1,
            generateTime: createTime.toISOString(),
         },
      };

      return {
         id: index + 1,
         userId: Math.floor(Math.random() * 10) + 1,
         projectName: mockProjectNames[index],
         supplierName: mockSuppliers[index],
         parseStatus: parseStatus,
         latestVersion: Math.floor(Math.random() * 5) + 1,
         createTime: createTime.toISOString(),
         updateTime: createTime.toISOString(),
         tenders: [bidDetail],
      };
   });
};

export const getMockDashboardList = (): ProjectItem[] => {
   return generateMockData();
};
