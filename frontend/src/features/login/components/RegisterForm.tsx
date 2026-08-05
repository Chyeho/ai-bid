import { Form, Input, Button } from 'antd';
import {
   UserOutlined,
   LockOutlined,
   PhoneOutlined,
} from '@ant-design/icons';
import { useStyles } from '../style';
import type { RegisterFormProps } from '../types';

export function RegisterForm({
   form,
   loading,
   onFinish,
   buttonClass,
}: RegisterFormProps) {
   const { theme } = useStyles();

   return (
      <Form
         form={form}
         name='register'
         onFinish={onFinish}
         layout='vertical'
         autoComplete='off'
         validateTrigger={['onBlur', 'onSubmit']}
      >
         <Form.Item
            name='username'
            label={<span>用户名</span>}
            rules={[{ required: true }]}
         >
            <Input
               prefix={
                  <UserOutlined style={{ color: theme.colorTextDescription }} />
               }
               placeholder='请输入用户名'
            />
         </Form.Item>

         <Form.Item
            name='phone'
            label={<span>手机号</span>}
            rules={[{ required: true, pattern: /^1[3-9]\d{9}$/ }]}
         >
            <Input
               prefix={
                  <PhoneOutlined
                     style={{ color: theme.colorTextDescription }}
                  />
               }
               placeholder='请输入手机号'
            />
         </Form.Item>

         <Form.Item
            name='password'
            label={<span>密码</span>}
            rules={[{ required: true, min: 6 }]}
         >
            <Input.Password
               prefix={
                  <LockOutlined style={{ color: theme.colorTextDescription }} />
               }
               placeholder='请输入密码'
            />
         </Form.Item>

         <Form.Item>
            <Button
               type='primary'
               htmlType='submit'
               loading={loading}
               className={buttonClass}
            >
               {loading ? '注册中...' : '注册'}
            </Button>
         </Form.Item>
      </Form>
   );
}
