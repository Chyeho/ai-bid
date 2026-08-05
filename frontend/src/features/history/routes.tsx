import type { RouteObject } from 'react-router-dom';
import { HistoryPage } from './HistoryPage';

export const historyRoutes: RouteObject[] = [
   { path: 'history', element: <HistoryPage /> },
];
