import React from 'react';
import { Spin } from 'antd';
import { LoadingOutlined } from '@ant-design/icons';

interface LoadingProps {
   loading: boolean;
   children?: React.ReactNode;
   description?: string;
   fullScreen?: boolean;
}

export const Loading: React.FC<LoadingProps> = ({
   loading,
   children,
   description = '加载中...',
   fullScreen = false,
}) => {
   return (
      <Spin
         spinning={loading}
         tip={children || fullScreen ? description : undefined}
         indicator={<LoadingOutlined style={{ fontSize: 24 }} spin />}
         style={
            fullScreen
               ? {
                    minHeight: '300px',
                    display: 'flex',
                    justifyContent: 'center',
                    alignItems: 'center',
                 }
               : {}
         }
      >
         {children}
      </Spin>
   );
};
