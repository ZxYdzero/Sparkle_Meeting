<script setup lang="ts">
import TauriCallView from "./components/TauriCallView.vue";
</script>

<template>
  <main class="container">
    <TauriCallView />
  </main>
</template>
<style>
:root {
  /* 腾讯会议风格字体 */
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC", "Hiragino Sans GB", "Microsoft YaHei", "Helvetica Neue", Helvetica, Arial, sans-serif;
  font-size: 14px;
  line-height: 1.5;
  font-weight: 400;

  /* 腾讯会议深色主题 */
  color: #ffffff;
  background: #1e1e1e;
  min-height: 100vh;

  /* 改进的渲染设置 */
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;

  /* 腾讯会议配色方案 */
  --tencent-primary: #006EFF;
  --tencent-primary-hover: #0052CC;
  --tencent-primary-light: rgba(0, 110, 255, 0.1);
  --tencent-success: #2CA91F;
  --tencent-warning: #FA8C16;
  --tencent-danger: #E74C3C;
  --tencent-info: #1890FF;

  /* 背景色 */
  --bg-primary: #1e1e1e;
  --bg-secondary: #2a2a2a;
  --bg-tertiary: #333333;
  --bg-elevated: #404040;
  --bg-overlay: rgba(0, 0, 0, 0.8);

  /* 文字颜色 */
  --text-primary: #ffffff;
  --text-secondary: #b3b3b3;
  --text-tertiary: #808080;
  --text-disabled: #4a4a4a;

  /* 边框颜色 */
  --border-primary: #404040;
  --border-secondary: #4a4a4a;
  --border-light: rgba(255, 255, 255, 0.1);

  /* 阴影 */
  --shadow-sm: 0 2px 8px rgba(0, 0, 0, 0.15);
  --shadow-md: 0 4px 16px rgba(0, 0, 0, 0.2);
  --shadow-lg: 0 8px 24px rgba(0, 0, 0, 0.3);
  --shadow-xl: 0 16px 32px rgba(0, 0, 0, 0.4);

  /* 圆角 */
  --radius-sm: 4px;
  --radius-md: 8px;
  --radius-lg: 12px;
  --radius-xl: 16px;

  /* 控制栏高度 */
  --control-bar-height: 60px;
  --control-button-size: 40px;
}

/* 腾讯会议全局样式 */
* {
  box-sizing: border-box;
}

body {
  margin: 0;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-family: inherit;
  overflow-x: hidden;
}

.container {
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
  background: var(--bg-primary);
}

/* 腾讯会议Logo样式 */
.logo {
  height: 32px;
  padding: 0.5rem;
  will-change: filter;
  transition: all 0.2s ease;
  filter: drop-shadow(0 2px 4px rgba(0, 0, 0, 0.3));
}

.logo.tauri:hover {
  transform: scale(1.05);
  filter: drop-shadow(0 4px 8px rgba(0, 110, 255, 0.4));
}

.row {
  display: flex;
  justify-content: center;
  gap: 0.5rem;
  flex-wrap: wrap;
}

/* 腾讯会议链接样式 */
a {
  font-weight: 500;
  color: var(--tencent-primary);
  text-decoration: none;
  transition: color 0.2s ease;
}

a:hover {
  color: var(--tencent-primary-hover);
}

h1 {
  text-align: center;
  color: var(--text-primary);
  font-weight: 600;
  font-size: 1.5rem;
  margin: 1rem 0;
  letter-spacing: 0.5px;
}

/* 腾讯会议输入框样式 */
input {
  border-radius: var(--radius-md);
  border: 1px solid var(--border-secondary);
  padding: 0.625rem 1rem;
  font-size: 0.875rem;
  font-weight: 400;
  font-family: inherit;
  color: var(--text-primary);
  background-color: var(--bg-secondary);
  transition: all 0.2s ease;
  outline: none;
}

input:hover {
  border-color: var(--border-primary);
  background-color: var(--bg-tertiary);
}

input:focus {
  border-color: var(--tencent-primary);
  background-color: var(--bg-tertiary);
  box-shadow: 0 0 0 2px var(--tencent-primary-light);
}

input::placeholder {
  color: var(--text-tertiary);
}

/* 腾讯会议按钮样式 */
button {
  border-radius: var(--radius-md);
  border: none;
  padding: 0.625rem 1.25rem;
  font-size: 0.875rem;
  font-weight: 500;
  font-family: inherit;
  color: white;
  background-color: var(--tencent-primary);
  cursor: pointer;
  transition: all 0.2s ease;
  outline: none;
  position: relative;
  overflow: hidden;
}

button:hover:not(:disabled) {
  background-color: var(--tencent-primary-hover);
  transform: translateY(-1px);
  box-shadow: var(--shadow-md);
}

button:active:not(:disabled) {
  transform: translateY(0);
  box-shadow: var(--shadow-sm);
}

button:disabled {
  background-color: var(--bg-tertiary);
  color: var(--text-disabled);
  cursor: not-allowed;
  transform: none;
  box-shadow: none;
}

button:disabled:hover {
  transform: none;
  box-shadow: none;
}

/* 腾讯会议主要按钮 */
.btn-primary {
  background-color: var(--tencent-primary);
}

.btn-primary:hover:not(:disabled) {
  background-color: var(--tencent-primary-hover);
}

/* 腾讯会议成功按钮 */
.btn-success {
  background-color: var(--tencent-success);
}

.btn-success:hover:not(:disabled) {
  background-color: #237A18;
  box-shadow: 0 4px 12px rgba(44, 169, 31, 0.3);
}

/* 腾讯会议警告按钮 */
.btn-warning {
  background-color: var(--tencent-warning);
  color: white;
}

.btn-warning:hover:not(:disabled) {
  background-color: #D97A0A;
  box-shadow: 0 4px 12px rgba(250, 140, 22, 0.3);
}

/* 腾讯会议危险按钮 */
.btn-danger {
  background-color: var(--tencent-danger);
}

.btn-danger:hover:not(:disabled) {
  background-color: #C0392B;
  box-shadow: 0 4px 12px rgba(231, 76, 60, 0.3);
}

#greet-input {
  margin-right: 0.5rem;
  min-width: 200px;
}

/* 腾讯会议响应式设计 */
@media (max-width: 768px) {
  .container {
    padding: 0;
  }

  h1 {
    font-size: 1.25rem;
    margin: 0.75rem 0;
  }

  .row {
    flex-direction: column;
    width: 100%;
    padding: 0 1rem;
  }

  input, button {
    width: 100%;
    margin: 0.25rem 0;
  }

  #greet-input {
    margin-right: 0;
  }
}

/* 腾讯会议动画效果 */
@keyframes fadeInUp {
  from {
    opacity: 0;
    transform: translateY(20px);
  }
  to {
    opacity: 1;
    transform: translateY(0);
  }
}

.tencent-fade-in {
  animation: fadeInUp 0.3s ease-out;
}

/* 腾讯会议滚动条样式 */
::-webkit-scrollbar {
  width: 6px;
  height: 6px;
}

::-webkit-scrollbar-track {
  background: var(--bg-secondary);
}

::-webkit-scrollbar-thumb {
  background: var(--border-primary);
  border-radius: 3px;
}

::-webkit-scrollbar-thumb:hover {
  background: var(--text-tertiary);
}

/* 腾讯会议选择框样式 */
select {
  border-radius: var(--radius-md);
  border: 1px solid var(--border-secondary);
  padding: 0.625rem 1rem;
  font-size: 0.875rem;
  font-weight: 400;
  font-family: inherit;
  color: var(--text-primary);
  background-color: var(--bg-secondary);
  transition: all 0.2s ease;
  outline: none;
  cursor: pointer;
}

select:hover {
  border-color: var(--border-primary);
  background-color: var(--bg-tertiary);
}

select:focus {
  border-color: var(--tencent-primary);
  background-color: var(--bg-tertiary);
  box-shadow: 0 0 0 2px var(--tencent-primary-light);
}

select option {
  background-color: var(--bg-secondary);
  color: var(--text-primary);
}
</style>