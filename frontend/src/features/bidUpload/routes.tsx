import type { RouteObject } from 'react-router-dom';
import { BidUploadPage } from './BidUploadPage';

export const uploadRoutes: RouteObject[] = [
   { path: '/upload/:projectId', element: <BidUploadPage /> },
];
