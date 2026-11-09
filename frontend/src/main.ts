import { createApp } from 'vue'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import * as ElementPlusIconsVue from '@element-plus/icons-vue'
import zhCn from 'element-plus/dist/locale/zh-cn.mjs'
import App from './App.vue'

// 创建应用实例
const app = createApp(App)

// 使用 Element Plus
app.use(ElementPlus, {
  locale: zhCn,
  size: 'default',
  zIndex: 3000,
})

// 注册所有 Element Plus 图标
for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
  app.component(key, component)
}

// 全局错误处理
app.config.errorHandler = (err, vm, info) => {
  console.error('全局错误:', err)
  console.error('错误信息:', info)
  // 这里可以添加错误上报逻辑
}

// 挂载应用
app.mount('#app')

// 开发环境配置
if (import.meta.env.DEV) {
  console.log('🚀 SecurePass 前端开发环境已启动')
  console.log('📱 当前环境:', import.meta.env.MODE)
  console.log('🌐 API 地址:', import.meta.env.VITE_API_URL || 'http://localhost:8000')
  console.log('🎯 开发服务器:', import.meta.env.VITE_DEV_SERVER_URL || 'http://localhost:3000')
}

// 生产环境配置
if (import.meta.env.PROD) {
  console.log('🏭 SecurePass 生产环境已启动')
  console.log('📦 构建版本:', import.meta.env.VITE_APP_VERSION || '1.0.0')
  console.log('🔒 生产模式已启用')
}