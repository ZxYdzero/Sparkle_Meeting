# SPK Meeting 客户端

基于 Vue.js 3 + TypeScript 构建的现代化在线会议客户端应用。

## 📋 功能特性

- 🎥 **多人视频通话** - 支持多人同时在线会议
- 🔊 **实时音频** - 高质量音频传输
- 📱 **响应式设计** - 适配桌面和移动设备
- 🎛️ **音视频控制** - 实时开关麦克风和摄像头
- 🏠 **房间系统** - 加入和创建会议房间
- 👥 **用户管理** - 显示在线用户列表
- 🎨 **现代UI** - 美观直观的用户界面

## 🏗️ 技术架构

### 核心技术栈

- **Vue.js 3** - 渐进式前端框架
- **TypeScript** - 类型安全的 JavaScript
- **Vite** - 快速的构建工具
- **WebRTC** - 实时音视频通信
- **WebSocket** - 信令通信

### 组件结构

```
src/
├── components/
│   └── CallView.vue      # 主要会议组件
├── App.vue              # 应用根组件
├── main.ts              # 应用入口
├── vite-env.d.ts        # Vite 类型定义
└── assets/              # 静态资源
```

## 🚀 快速开始

### 环境要求

- Node.js 16+
- npm 或 yarn
- 现代浏览器（支持 WebRTC）

### 环境要求

- Node.js 16+
- Rust 1.70+
- npm 或 yarn
- 现代浏览器（支持 WebRTC）

### 安装和运行

1. **安装依赖**
```bash
npm install
```

2. **启动开发模式**
```bash
npm run tauri dev
```

3. **构建桌面应用**
```bash
# 构建前端并打包桌面应用
npm run tauri build

# 或者使用 Cargo 命令
cd src-tauri
cargo tauri build
```

4. **仅构建前端（用于 Web 部署）**
```bash
npm run build
npm run preview
```

### 构建产物

- **桌面应用**: `src-tauri/target/release/bundle/`
- **Web 应用**: `dist/` 目录

## 🎯 使用指南

### 基本操作

1. **加入会议**
   - 输入房间名称（如：`room1`）
   - 输入用户名（如：`user1`）
   - 点击"加入房间"按钮
   - 允许浏览器访问摄像头和麦克风

2. **音视频控制**
   - **静音/取消静音** - 控制麦克风开关
   - **开启/关闭视频** - 控制摄像头开关

3. **离开会议**
   - 点击"离开"按钮
   - 关闭所有音视频连接

## 🔧 配置说明

### 服务器配置

在 `CallView.vue` 中可以修改服务器地址：

```typescript
// 可配置项
const SIGNALING_WS = 'ws://127.0.0.1:8080/ws/';  // 信令服务器地址
const ROOMS_API = 'http://127.0.0.1:8080/rooms'; // 房间API地址
const ICE_SERVERS = [                           // STUN/TURN 服务器
  { urls: 'stun:stun.l.google.com:19302' }
];
```

## 📱 移动端适配

应用采用响应式设计，自动适配不同屏幕尺寸：

- **桌面端** - 完整功能界面
- **平板端** - 调整布局和按钮大小
- **手机端** - 垂直布局，优化触摸操作

## 🐛 故障排除

### 常见问题

1. **无法访问摄像头/麦克风**
   - 检查浏览器权限设置
   - 确认设备未被其他应用占用
   - 尝试使用其他浏览器

2. **连接不上服务器**
   - 检查服务器是否正在运行
   - 确认网络连接正常
   - 检查防火墙设置

3. **视频画面卡顿**
   - 检查网络带宽
   - 尝试关闭视频只使用音频
   - 减少同时连接的用户数量

## 🎨 开发和构建

### 开发环境

推荐使用以下 IDE 和插件：

- [VS Code](https://code.visualstudio.com/) + [Volar](https://marketplace.visualstudio.com/items?itemName=Vue.volar) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

### TypeScript 支持

为了获得更好的 TypeScript 支持，可以启用 Volar 的 Take Over 模式：

1. 在 VS Code 中运行 `Extensions: Show Built-in Extensions`
2. 找到 `TypeScript and JavaScript Language Features`，右键选择 `Disable (Workspace)`
3. 重新加载 VS Code 窗口

### 构建和部署

```bash
# 开发模式
npm run dev

# 构建生产版本
npm run build

# 预览生产版本
npm run preview
```

## 📊 浏览器兼容性

| 浏览器 | 版本要求 | WebRTC 支持 |
|--------|----------|-------------|
| Chrome | 60+ | ✅ 完全支持 |
| Firefox | 55+ | ✅ 完全支持 |
| Safari | 11+ | ✅ 支持 |
| Edge | 79+ | ✅ 支持 |

## 📄 许可证

MIT License
