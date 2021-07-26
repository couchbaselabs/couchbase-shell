import { createApp } from 'vue'
import App from './App.vue'
import axios from 'axios'
import VueAxios from 'vue-axios'
import UUID from "vue-uuid";
import router from './router'

createApp(App).use(router).use(VueAxios, axios, UUID).mount('#app')
