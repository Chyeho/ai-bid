# Frontend — React 前端界面

智能标书审核系统的 Web 前端。

## 技术栈

- React 18 + TypeScript
- Vite 构建工具
- pnpm 包管理

## 构建与运行

```bash
pnpm install
pnpm dev        # 开发服务器 (localhost:5173)
pnpm build      # 生产构建
pnpm preview    # 预览生产构建
```

## API 约定

- 后端 API 基础路径：`http://localhost:3000/api`
- 请求/响应格式：JSON
- API 文档：待补充（建议使用 OpenAPI/Swagger）

## 目录结构

```
src/
├── api/          # API 请求封装
├── components/   # 通用组件
├── pages/        # 页面组件
├── hooks/        # 自定义 Hooks
├── stores/       # 状态管理
├── types/        # TypeScript 类型定义
└── utils/        # 工具函数
```

## 注意事项

- 组件使用函数式 + Hooks
- 类型定义与后端 API 响应保持一致
- 开发时注意 CORS 配置
