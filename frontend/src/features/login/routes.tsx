import type { RouteObject } from 'react-router-dom';
import { LoginPage } from './LoginPage';

export const loginRoutes: RouteObject[] = [
   { path: '/login', element: <LoginPage /> },
];
