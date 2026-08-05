# 智能标书审核系统

## 版本信息
- 版本：2026.3.6_v0.6
- 开发时间：2026年3月6日

## 本次更新内容
### 1. 标准库管理代码重构与组件分离
将`BidLibraryPage.tsx` 重构拆分为多个小组件，提升代码可维护性：

#### 1.1 主页面优化
- **BidLibraryPage.tsx**：代码精简
- 主页面仅负责：渲染整个页面布局、获取数据、状态整合
- 业务逻辑和UI组件完全分离

#### 1.2 新增组件
- **SearchBar.tsx**：搜索框和上传按钮组件
- **CategoryTabs.tsx**：分类标签栏组件
- **StatCard.tsx**：统计卡片组件
- **FilterBar.tsx**：筛选栏组件
- **FileTable.tsx**：文件表格组件
- **UploadModal.tsx**：上传弹窗组件
- **EditModal.tsx**：编辑弹窗组件
- **ViewModal.tsx**：查看弹窗组件

#### 1.3 新增模块
- **constants.ts**：提取分类相关常量
- **mockData.ts**：提取模拟数据
- **hooks/useBidLibrary.ts**：业务逻辑自定义 Hook，管理状态、筛选、分页等核心逻辑

### 2. 代码注释完善
按照注释要求文档和前端常用注释规范，为 `bidLibrary` 文件夹内所有代码添加清晰的中文注释：

- 每个组件添加功能说明和 Props 注释
- 自定义 Hook 添加完整的业务逻辑注释
- 常量和数据文件添加字段说明注释
- 样式文件按区块分组添加详细注释
- 便于他人阅读理解和后续维护

### 3. 样式与体验优化
- 移除页面灰色背景，视觉更清爽
- 增强统计卡片阴影效果，层次更分明
- 保持响应式布局和暗色模式完美适配

### 4. 功能修复
- 修复编辑功能：编辑文件信息后文件名、文件类型和适用范围正常显示
- 重构 EditModal 组件，优化表单数据初始化流程

## 网站定位
智能标书审核系统是一个专为东莞理工学院财务部设计的在线平台，旨在简化标书审核流程，提高审核效率和准确性。系统采用现代化的前端技术栈，提供直观友好的用户界面，支持智能审核功能。

## 已实现的基本功能
1. **登录系统**：包含用户名密码登录，支持默认账号（admin/123456）
2. **深色模式**：支持明亮/暗黑两种主题模式切换，所有组件完美适配
3. **响应式布局**：自适应不同屏幕尺寸，窗口缩窄时自动缩小侧边栏，移动端自动切换为移动视图
4. **主布局**：包含侧边栏导航、顶部栏（主题切换、通知中心、用户信息）
5. **通知中心**：显示未读消息数量，支持一键已读、查看更多、未读/全部标签切换
6. **功能页面**：
   - 工作台
   - 标书上传
   - 审核列表
   - **标准库管理**：
     - 支持文件分类标签切换（全部、制度文件、价格标准、供应商名录、合同模板、案例库、其他）
     - 多条件筛选（搜索关键词、适用范围、状态、上传时间范围）
     - 文件数据统计卡片（两列布局）
     - 文件类型颜色区分（不同分类显示不同颜色标签，暗色模式适配）
     - 状态标签（启用/停用，暗色模式适配）
     - 操作栏（查看、下载、编辑、删除）
     - 编辑功能（可修改文件名、类型、适用范围、描述、状态）
     - 分页功能
     - 暗色模式完美适配
     - 响应式布局
   - 历史记录
   - 系统设置

## 运行代码的方法

### 环境要求
- Node.js 16.0 或更高版本
- npm 7.0 或更高版本

### 安装步骤
1. **克隆项目**（如果尚未克隆）
   ```bash
   git clone https://github.com/Willfulchip/bid-audit.git
   cd bid-audit
   ```

2. **安装依赖**
   ```bash
   npm install
   ```

3. **启动开发服务器**
   ```bash
   npm run dev
   ```
   开发服务器将在 http://localhost:5173/ 启动（如果端口被占用，会自动使用其他端口）

   默认账号为：**admin**
   密码为：**123456**
   
4. **构建生产版本**
   ```bash
   npm run build
   ```
   构建产物将生成在 `dist` 目录中

5. **预览生产构建**
   ```bash
   npm run preview
   ```

## 简要的使用说明

### 登录系统
1. 打开浏览器，访问 http://localhost:5173/login
2. 输入默认账号：用户名 `admin`，密码 `123456`
3. 点击「登录」按钮进入系统

### 系统导航
- **侧边栏**：提供主要功能模块的导航
- **顶部栏**：显示当前页面路径、主题切换按钮、通知中心、用户信息
- **通知中心**：点击铃铛图标查看系统通知，支持标记已读

### 主题切换
点击顶部栏的月亮/太阳图标可切换明亮/暗黑主题模式

### 标准库管理
1. **查看文件**：点击「查看」按钮查看文件详情
2. **下载文件**：点击「下载」按钮下载文件
3. **编辑文件**：点击「编辑」按钮修改文件信息（文件名、类型、适用范围、描述、状态）
4. **删除文件**：点击「删除」按钮删除文件
5. **分类筛选**：点击顶部标签切换不同分类的文件
6. **多条件筛选**：使用搜索框、下拉选择器和日期选择器进行精准筛选
7. **查看统计**：右侧卡片显示文件统计数据

## 代码目录结构说明

```
bid-audit/
├── public/                 # 静态资源
├── src/
│   ├── api/                # API配置
│   │   ├── request.ts      # Axios实例配置
│   │   └── types.ts        # API类型定义
│   ├── app/               # 应用核心配置
│   │   ├── RouteGuard.tsx # 路由守卫
│   │   └── router.tsx     # 路由配置
│   ├── assets/            # 静态资源
│   ├── components/        # 通用组件
│   │   ├── layout/        # 布局组件
│   │   │   ├── Header.tsx              # 顶部栏
│   │   │   ├── Layout.css              # 布局样式
│   │   │   ├── MainLayout.tsx          # 主布局
│   │   │   ├── NotificationCenter.tsx   # 通知中心
│   │   │   └── Sidebar.tsx             # 侧边栏
│   │   └── theme-provider.tsx        # 主题提供者
│   ├── features/          # 功能模块
│   │   ├── bidLibrary/     # 标准库管理
│   │   │   ├── api/         # 标准库API
│   │   │   │   ├── knowledgeFile.ts  # 文件API
│   │   │   │   └── types.ts          # 类型定义
│   │   │   ├── components/   # 标准库组件
│   │   │   │   ├── SearchBar.tsx      # 搜索栏组件
│   │   │   │   ├── CategoryTabs.tsx   # 分类标签组件
│   │   │   │   ├── StatCard.tsx        # 统计卡片组件
│   │   │   │   ├── FilterBar.tsx       # 筛选栏组件
│   │   │   │   ├── FileTable.tsx       # 文件表格组件
│   │   │   │   ├── UploadModal.tsx     # 上传弹窗组件
│   │   │   │   ├── EditModal.tsx       # 编辑弹窗组件
│   │   │   │   └── ViewModal.tsx       # 查看弹窗组件
│   │   │   ├── hooks/        # 自定义 Hooks
│   │   │   │   └── useBidLibrary.ts    # 标准库业务逻辑 Hook
│   │   │   ├── constants.ts  # 分类相关常量
│   │   │   ├── mockData.ts   # 模拟数据
│   │   │   ├── BidLibraryPage.tsx # 标准库页面
│   │   │   ├── style.ts          # 标准库样式（createStyles）
│   │   │   └── routes.tsx    # 标准库路由
│   │   ├── bidReview/      # 审核列表
│   │   ├── bidUpload/      # 标书上传
│   │   ├── dashboard/      # 工作台
│   │   ├── history/        # 历史记录
│   │   │   ├── api/         # 历史记录API
│   │   │   ├── HistoryPage.tsx # 历史记录页面
│   │   │   └── routes.tsx    # 历史记录路由
│   │   ├── login/          # 登录页面
│   │   │   ├── api/         # 登录API
│   │   │   ├── LoginPage.css # 登录样式
│   │   │   ├── LoginPage.tsx # 登录页面
│   │   │   └── routes.tsx    # 登录路由
│   │   └── settings/       # 系统设置
│   ├── hooks/             # 自定义Hooks
│   │   └── useMediaQuery.ts # 响应式Hook
│   ├── lib/               # 工具库
│   │   ├── queryClient.ts  # React Query配置
│   │   └── utils.ts        # 工具函数
│   ├── store/             # Redux状态管理
│   │   ├── slices/         # 状态切片
│   │   │   └── authSlice.ts # 认证状态
│   │   └── index.ts        # 存储配置
│   ├── styles/            # 样式文件
│   ├── theme/             # 主题配置
│   │   └── constants.ts    # 主题常量
│   ├── App.css            # 应用样式
│   ├── App.tsx            # 应用入口组件
│   ├── index.css          # 全局样式
│   └── main.tsx           # 应用入口文件
├── package.json           # 项目配置
└── vite.config.ts         # Vite配置
```

## 技术栈
- **前端框架**：React 19 + TypeScript
- **UI组件库**：Ant Design 6.x
- **样式方案**：antd-style（createStyles）
- **状态管理**：Redux Toolkit
- **路由**：React Router 7.x
- **HTTP客户端**：Axios
- **构建工具**：Vite
- **图标库**：Lucide React, Ant Design Icons
- **图表库**：Recharts
- **数据请求**：React Query

## 开发信息
- 开发时间：2026年3月6日
- 版本：2026.3.6_v0.6
- 状态：开发中
- 开发人：ZQS74
