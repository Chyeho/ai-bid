import { createSlice } from '@reduxjs/toolkit';

export interface UserInfo {
   id: number;
   username: string;
   realName: string;
}

export interface AuthState {
   token: string | null;
   userInfo: UserInfo | null;
   isAuthenticated: boolean;
}

const clearAllStorage = () => {
   localStorage.removeItem('token');
   localStorage.removeItem('userInfo');
   sessionStorage.removeItem('token');
   sessionStorage.removeItem('userInfo');
};

const getStoredToken = () =>
   localStorage.getItem('token') || sessionStorage.getItem('token');
const getStoredUserInfo = () =>
   localStorage.getItem('userInfo') || sessionStorage.getItem('userInfo');

const token = getStoredToken();
const userInfoStr = getStoredUserInfo();

const initialState: AuthState = {
   token: token,
   userInfo: userInfoStr ? JSON.parse(userInfoStr) : null,
   isAuthenticated: !!token,
};

const authSlice = createSlice({
   name: 'auth',
   initialState,
   reducers: {
      setCredentials: (
         state,
         action: {
            payload: {
               token: string;
               userInfo: UserInfo;
               rememberMe?: boolean;
            };
         }
      ) => {
         const { token, userInfo, rememberMe } = action.payload;
         state.token = token;
         state.userInfo = userInfo;
         state.isAuthenticated = true;

         clearAllStorage();

         if (rememberMe) {
            localStorage.setItem('token', token);
            localStorage.setItem('userInfo', JSON.stringify(userInfo));
         } else {
            sessionStorage.setItem('token', token);
            sessionStorage.setItem('userInfo', JSON.stringify(userInfo));
         }
      },
      logout: (state) => {
         state.token = null;
         state.userInfo = null;
         state.isAuthenticated = false;
         clearAllStorage();
      },
      restoreAuth: (state) => {
         const token =
            localStorage.getItem('token') || sessionStorage.getItem('token');
         const userInfoStr =
            localStorage.getItem('userInfo') ||
            sessionStorage.getItem('userInfo');

         if (token && userInfoStr) {
            try {
               const userInfo = JSON.parse(userInfoStr);
               state.token = token;
               state.userInfo = userInfo;
               state.isAuthenticated = true;
            } catch (error) {
               console.error('Failed to parse user info:', error);
               clearAllStorage();
            }
         }
      },
   },
});

export const { setCredentials, logout, restoreAuth } = authSlice.actions;
export default authSlice.reducer;
