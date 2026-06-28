import type { ThemeConfig } from 'antd';
import { theme } from 'antd';

export const COLORS = {
   // --- 主色系列 ---
   primary: '#2E7D32', // 学校绿 (主色)
   primaryHover: '#43A047', // 中绿色 (悬停)
   primaryLight: '#E8F5E9', // 浅绿色 (浅色模式背景/选中)
   primaryDark: '#1B5E20', // 深绿 (深色模式强调)

   // --- 文字色 ---
   textMain: '#333333', // 深灰色 (主要文字-浅色模式)
   textSecondary: '#666666', // 中灰色 (次要文字-浅色模式)
   textMainDark: 'rgba(255, 255, 255, 0.85)', // 主要文字-深色模式
   textSecondaryDark: 'rgba(255, 255, 255, 0.65)', // 次要文字-深色模式

   // --- 背景色 ---
   bgLayoutLight: '#F8F9FA', // 浅灰色 (系统背景-浅色)
   bgLayoutDark: '#141414', // 深色模式背景

   // --- 功能色 ---
   border: '#E0E0E0', // 边框灰 (浅色)
   borderDark: '#424242', // 边框灰 (深色)
   error: '#D32F2F', // 严重问题 (红色)
   warning: '#ED6C02', // 警告 (橙色)
   success: '#43A047', // 通过 (绿色)
};

export const getThemeConfig = (isDark: boolean): ThemeConfig => {
   return {
      // 根据模式选择算法
      algorithm: isDark ? theme.darkAlgorithm : theme.defaultAlgorithm,

      token: {
         // --- 基础色彩映射 ---
         colorPrimary: COLORS.primary,
         colorSuccess: COLORS.success,
         colorWarning: COLORS.warning,
         colorError: COLORS.error,

         // --- 文字与背景 (动态) ---
         colorTextBase: isDark ? COLORS.textMainDark : COLORS.textMain,
         colorTextDescription: isDark
            ? COLORS.textSecondaryDark
            : COLORS.textSecondary,
         colorBgLayout: isDark ? COLORS.bgLayoutDark : COLORS.bgLayoutLight,
         colorBgContainer: isDark ? '#1d1d1d' : '#FFFFFF',
         colorBorder: isDark ? COLORS.borderDark : COLORS.border,

         // 深色模式下使用半透明主色，避免浅绿色刺眼
         colorFillAlter: isDark
            ? 'rgba(46, 125, 50, 0.15)'
            : COLORS.primaryLight,

         // --- 圆角规范 (规范 4.1) ---
         borderRadius: 8,
         borderRadiusSM: 4, // 用于输入框等小组件

         // --- 字体规范 (规范 3.1) ---
         fontFamily: 'PingFang SC, Inter, system-ui, -apple-system, sans-serif',
         fontSize: 14,
      },

      components: {
         Breadcrumb: {
            // 历史层级（可点击项）的颜色：浅色用主色（绿），深色用亮绿
            linkColor: isDark ? '#81C784' : COLORS.primary,
            linkHoverColor: isDark ? '#A5D6A7' : COLORS.primaryHover,
            // 当前所在页面（最后一项）的颜色：浅色黑，深色白
            lastItemColor: isDark ? COLORS.textMainDark : COLORS.textMain,
            separatorColor: isDark
               ? COLORS.textSecondaryDark
               : COLORS.textSecondary,
         },
         Menu: {
            itemBg: 'transparent',
            // 深色模式：半透明绿色背景；浅色模式：浅绿背景
            itemSelectedBg: isDark
               ? 'rgba(46, 125, 50, 0.2)'
               : COLORS.primaryLight,
            // 深色模式：亮绿色文字；浅色模式：主绿色
            itemSelectedColor: isDark ? '#81C784' : COLORS.primary,
            itemActiveBg: isDark
               ? 'rgba(255, 255, 255, 0.05)'
               : COLORS.primaryLight,
            itemHoverBg: isDark
               ? 'rgba(255, 255, 255, 0.05)'
               : COLORS.primaryLight,
            itemBorderRadius: 0, // 侧边栏通常直角或小圆角
            itemMarginInline: 0, // 消除左右边距，使选中态铺满
         },

         // --- 表格定制 (规范 5.3) ---
         Table: {
            // 深色模式：深灰背景；浅色模式：浅绿背景
            headerBg: isDark ? '#1d1d1d' : COLORS.primaryLight,
            headerColor: isDark ? COLORS.textMainDark : COLORS.textMain,
            headerBorderRadius: 0,
            rowHoverBg: isDark
               ? 'rgba(255, 255, 255, 0.04)'
               : COLORS.primaryLight,
         },

         // --- 按钮定制 ---
         Button: {
            colorPrimaryHover: COLORS.primaryHover,
            borderRadius: 8,

            colorLink: COLORS.primary, // 默认颜色
            colorLinkHover: COLORS.primaryHover, // 悬停颜色
         },

         // --- 输入框定制 ---
         Input: {
            controlHeight: 40,
            colorBorder: isDark ? COLORS.borderDark : COLORS.border,
            borderRadius: 4,
         },

         // --- 卡片定制 ---
         Card: {
            boxShadow: isDark
               ? '0 4px 12px rgba(0,0,0,0.5)'
               : '0 2px 8px rgba(0,0,0,0.05)',
            borderRadiusLG: 12,
         },
         Collapse: {
            headerPadding: '8px 12px',
         },
      },
   };
};

export const themeConfig: ThemeConfig = getThemeConfig(false);
