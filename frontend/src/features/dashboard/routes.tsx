import type { RouteObject } from 'react-router-dom';
import { DashboardPage } from './DashboardPage';

export const dashboardRoutes: RouteObject[] = [
   { path: '/dashboard', element: <DashboardPage /> },
];
