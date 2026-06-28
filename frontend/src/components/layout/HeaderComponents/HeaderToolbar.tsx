import React from 'react';
import { Space, Avatar, Button, Dropdown, type MenuProps } from 'antd';
import { useSelector, useDispatch } from 'react-redux';
import { useNavigate } from 'react-router-dom';
import { Moon, Sun, User, LogOut } from 'lucide-react';

import type { RootState } from '@/store';
import { logout } from '@/store/slices/authSlice';
import { loginApi } from '@/features/login/api/login';
import { useTheme } from '../../theme-provider';
import { useHeaderStyle } from '../style';

interface HeaderToolbarProps {
   isMobile: boolean;
}

export const HeaderToolbar: React.FC<HeaderToolbarProps> = ({ isMobile }) => {
   const { theme: antdTheme } = useHeaderStyle();
   const dispatch = useDispatch();
   const navigate = useNavigate();
   const { theme, setTheme } = useTheme();
   const { userInfo } = useSelector((state: RootState) => state.auth);

   const userMenuItems: MenuProps['items'] = [
      {
         key: 'logout',
         icon: <LogOut size={14} />,
         label: '退出登录',
         style: { fontSize: '1.2rem' },
         danger: true,
         onClick: async () => {
            try {
               await loginApi.logout();
            } catch (error) {
               console.error('退出登录失败，请稍后重试', error);
            } finally {
               dispatch(logout());
               navigate('/login', { replace: true });
            }
         },
      },
   ];

   return (
      <Space size='small' align='center'>
         <Button
            type='text'
            icon={theme === 'dark' ? <Sun size={16} /> : <Moon size={16} />}
            onClick={() => setTheme(theme === 'dark' ? 'light' : 'dark')}
            style={{ color: antdTheme.colorTextBase }}
         />

         <Dropdown
            menu={{ items: userMenuItems }}
            align={{ offset: [0, -8] }}
            trigger={['hover']}
            placement='bottomRight'
            arrow
         >
            <Space style={{ cursor: 'pointer', padding: '0 4px' }}>
               <Avatar
                  size={isMobile ? 'default' : 'small'}
                  style={{
                     backgroundColor: antdTheme.colorPrimary,
                  }}
               >
                  {userInfo?.realName?.charAt(0) || <User size={14} />}
               </Avatar>

               {!isMobile && (
                  <span
                     style={{
                        fontSize: 12,
                        color: antdTheme.colorTextBase,
                        lineHeight: 1,
                        display: 'inline-flex',
                        alignItems: 'center',
                     }}
                  >
                     {userInfo?.realName || '未知用户'}
                  </span>
               )}
            </Space>
         </Dropdown>
      </Space>
   );
};
