import type { RouteObject } from 'react-router-dom';
import { BidLibraryPage } from './BidLibraryPage';

export const libraryRoutes: RouteObject[] = [
   { path: '/library', element: <BidLibraryPage /> },
];
