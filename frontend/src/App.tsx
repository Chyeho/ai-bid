import React from 'react';
import { RouterProvider } from 'react-router-dom';
import { Provider as ReduxProvider } from 'react-redux';
import { QueryClientProvider } from '@tanstack/react-query';
import { ConfigProvider, App as AntdApp } from 'antd';
import zhCN from 'antd/locale/zh_CN';

import { store } from './store';
import queryClient from './lib/queryClient';
import { router } from './app/router';
import { getThemeConfig } from './theme/constants';

import { ThemeProvider, useTheme } from './components/theme-provider';

const AppContent: React.FC = () => {
   const { theme } = useTheme();

   // 判断是否为暗色模式（处理 system 逻辑）
   const isDark =
      theme === 'dark' ||
      (theme === 'system' &&
         window.matchMedia('(prefers-color-scheme: dark)').matches);

   return (
      <ReduxProvider store={store}>
         <QueryClientProvider client={queryClient}>
            <ConfigProvider locale={zhCN} theme={getThemeConfig(isDark)}>
               <AntdApp>
                  <RouterProvider router={router} />{' '}
               </AntdApp>
            </ConfigProvider>
         </QueryClientProvider>
      </ReduxProvider>
   );
};

const App: React.FC = () => {
   return (
      <ThemeProvider defaultTheme='system' storageKey='vite-ui-theme'>
         <AppContent />
      </ThemeProvider>
   );
};

export default App;
