import { createRouter, createWebHistory } from 'vue-router'
import Notebooks from "../components/Notebooks.vue";
import Notebook from "../components/Notebook.vue";

const routes = [
  {
    path: '/notebooks',
    component: Notebooks,
  },
  {
    path: '/notebook/:id',
    component: Notebook
  }
]

const router = createRouter({
  history: createWebHistory(process.env.BASE_URL),
  routes
})

export default router
