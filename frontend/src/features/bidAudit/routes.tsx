import type { RouteObject } from 'react-router-dom';
import { DetailPage } from './pages/Detail/DetailPage';
import { IssueListPage } from './pages/IssueList/IssueListPage';
import { ReportPage } from './pages/Report/ReportPage';
import { BidAuditList } from './BidAuditList';

export const bidAuditRoutes: RouteObject[] = [
   {
      path: 'bidReview',
      children: [
         { index: true, element: <BidAuditList /> },
         {
            path: 'detail/:id',
            element: <DetailPage />,
         },
         {
            path: 'issues/:id',
            element: <IssueListPage />,
         },
         {
            path: 'report/:id',
            element: <ReportPage />,
         },
      ],
   },
];
