import { createApp } from 'vue'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import * as ElementPlusIconsVue from '@element-plus/icons-vue'
import zhCn from 'element-plus/dist/locale/zh-cn.mjs'
import App from './App.vue'
import PasswordManager from './components/PasswordManager.vue'
import { installMockTauri } from './dev-mock'

installMockTauri()

const which = location.hash.replace('#', '')
const Root = which === 'app' ? App : PasswordManager

const app = createApp(Root as any)
app.use(ElementPlus, { locale: zhCn, size: 'default', zIndex: 3000 })
for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
  app.component(key, component as any)
}
app.mount('#app')
