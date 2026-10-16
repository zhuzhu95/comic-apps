import { createRouter, createWebHashHistory } from "vue-router";

const routes = [
  { path: "/", component: () => import("../views/Library.vue") },
  { path: "/reader/:id", component: () => import("../views/Reader.vue") },
  { path: "/settings", component: () => import("../views/Settings.vue") },
];

export default createRouter({
  history: createWebHashHistory(),
  routes,
});
