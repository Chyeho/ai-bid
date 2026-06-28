import { createStyles } from 'antd-style';

export const useStyles = createStyles(({ css, token }) => ({
   container: css`
      display: flex;
      flex-direction: column;
      height: 100%;
      border-radius: 12px;
      overflow: hidden;
      box-shadow: ${token.boxShadow};
      background: ${token.colorBgContainer};
      border: 1px solid ${token.colorBorderSecondary};
   `,
   header: css`
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 10px 12px;
      background: linear-gradient(
         135deg,
         ${token.colorPrimary} 0%,
         ${token.colorPrimaryHover} 100%
      );
      flex-shrink: 0;
   `,
   messageList: css`
      flex: 1;
      overflow-y: auto;
      padding: 14px 12px;
      background: ${token.colorBgLayout};
   `,
   footer: css`
      padding: 6px 12px 12px;
      border-top: 1px solid ${token.colorBorderSecondary};
      background: ${token.colorBgContainer};
      flex-shrink: 0;
   `,
   optionsRow: css`
      display: flex;
      justify-content: flex-end;
      margin-bottom: 6px;
   `,
   kbLabel: css`
      display: flex;
      align-items: center;
      gap: 4px;
      cursor: pointer;
   `,
   inputRow: css`
      display: flex;
      gap: 8px;
      align-items: flex-end;
   `,
   sendBtn: css`
      border-radius: 8px;
      height: 36px;
      width: 36px;
      flex-shrink: 0;
      background-color: #52c41a;
      border-color: #52c41a;
   `,
   centered: css`
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      height: 100%;
      padding: 20px;
   `,
   typing: css`
      display: flex;
      align-items: center;
      padding: 6px 12px;
   `,
}));
