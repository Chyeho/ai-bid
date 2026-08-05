import { Tabs } from 'antd';
import type { FormInstance } from 'antd';

import { useStyles } from '../style';

import { DocumentIllustration } from './DocumentIllustration';
import { LogoIcon } from './LogoIcon';
import { LoginForm } from './LoginForm';
import { RegisterForm } from './RegisterForm';
import type { LoginFormValues, RegisterFormValues } from '../types';

interface LoginView {
   activeTab: string;
   setActiveTab: (tab: string) => void;
   loginLoading: boolean;
   registerLoading: boolean;
   loginForm: FormInstance<LoginFormValues>;
   registerForm: FormInstance<RegisterFormValues>;
   onLoginFinish: (values: LoginFormValues) => void;
   onRegisterFinish: (values: RegisterFormValues) => void;
}

export function LoginView({
   activeTab,
   setActiveTab,
   loginLoading,
   registerLoading,
   loginForm,
   registerForm,
   onLoginFinish,
   onRegisterFinish,
}: LoginView) {
   const { styles, theme: tokenTheme } = useStyles();

   return (
      <div className={styles.loginContainer}>
         {/* --- 左侧面板：品牌与插图 --- */}
         <div className={styles.loginLeftPanel}>
            <div className={styles.loginBrandContent}>
               <h1 className={styles.loginBrandTitle}>
                  {<LogoIcon />}智能标书审核系统
               </h1>
               <div className={styles.loginIllustration}>
                  <DocumentIllustration tokenTheme={tokenTheme} />
               </div>
               <p className={styles.loginBrandSubtitle}>
                  AI赋能标书智能审核 助力财务合规管控
               </p>
            </div>
            <div className={styles.loginBrandFooter}>
               东莞理工学院财务部·智能审核平台
            </div>
         </div>

         <div className={styles.loginRightPanel}>
            <div className={styles.loginCard}>
               <div className={styles.loginCardHeader}>
                  <div className={styles.loginLogoWrapper}>
                     <LogoIcon />
                     <h2 className={styles.loginCardTitle}>智能标书审核系统</h2>
                  </div>
                  <Tabs
                     activeKey={activeTab}
                     onChange={setActiveTab}
                     centered
                     items={[
                        { key: 'login', label: '登录' },
                        { key: 'register', label: '注册' },
                     ]}
                  />
               </div>

               {activeTab === 'login' ? (
                  <LoginForm
                     form={loginForm}
                     loading={loginLoading}
                     onFinish={onLoginFinish}
                     buttonClass={styles.loginButton}
                  />
               ) : (
                  <RegisterForm
                     form={registerForm}
                     loading={registerLoading}
                     onFinish={onRegisterFinish}
                     buttonClass={styles.loginButton}
                  />
               )}
            </div>
         </div>
      </div>
   );
}
