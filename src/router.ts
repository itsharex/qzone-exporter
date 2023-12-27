import { createRouter, createWebHashHistory, RouterOptions, Router, RouteRecordRaw } from 'vue-router'

const routes: RouteRecordRaw[] = [
    { 
        path: '/', 
        name: 'Home', 
        component: () => import("./views/Photo.vue"), 
        meta: {
            keepAlive: true,
        }
    },
    { 
        path: '/task', 
        name: 'Task', 
        component: () => import("./views/Task.vue"),
        meta: {
            keepAlive: true,
        }
    },
]

// RouterOptions是路由选项类型
const options: RouterOptions = {
 history: createWebHashHistory(),
 routes,
}

// Router是路由对象类型
const router: Router = createRouter(options)

export default router