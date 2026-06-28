/**
 * Logo图标组件
 * @param color Logo背景颜色
 * @param textColor Logo文字颜色
 */
import bidAuditLogo from '@/assets/bid-audit.svg';

function LogoIcon() {
   return (
      <img src={bidAuditLogo} width='40' height='40' alt='智能标书审核系统' />
   );
}

export { LogoIcon };
