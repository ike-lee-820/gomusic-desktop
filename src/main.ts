import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import router from './router'
import '@fontsource-variable/noto-sans-sc/index.css'
import '@fontsource-variable/noto-serif-sc/index.css'
import './styles/tokens.css'
import './styles/global.css'

createApp(App).use(createPinia()).use(router).mount('#app')
