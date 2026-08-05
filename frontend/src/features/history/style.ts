import { createStyles } from 'antd-style';

export const useStyles = createStyles(({ css, token }) => ({
   pageContainer: css`
      display: flex;
      flex-direction: column;
      gap: 0.5rem;
      height: 100%;

      .ant-tabs-tab-btn {
         font-size: 1.2rem;
      }
   `,

   cardWrapper: css`
      background-color: ${token.colorBgContainer};
      border-radius: ${token.borderRadiusLG}px;
      padding: 1rem;
      box-shadow: 0 1px 2px 0 rgba(0, 0, 0, 0.03);
   `,

   filterForm: css`
      .ant-form-item {
         margin-bottom: 16px;
      }
   `,

   tableWrapper: css`
      .ant-table {
         font-size: 1.2rem;
      }

      .ant-table-thead > tr > th {
         background-color: ${token.colorFillAlter} !important;
         color: ${token.colorText};
         font-weight: 600;
         font-size: 1.3rem;
         padding: 8px 12px;
      }

      .ant-table-tbody > tr > td {
         padding: 8px 12px;
      }

      .action-space {
         gap: 4px;

         button {
            font-size: 1.2rem;
            padding: 6px 8px;
         }
      }
   `,

   auditResultsTag: css`
      text-align: center;
      font-size: 1.2rem;
      font-weight: 500;
      letter-spacing: 1px;
   `,
}));
