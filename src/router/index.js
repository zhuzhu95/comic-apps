import { createRouter, createWebHashHistory } from "vue-router";

const routes = [
  { path: "/", component: () => import("../views/Library.vue") },
  { path: "/reader/:id", component: () => import("../views/Reader.vue") },
];

export default createRouter({
  history: createWebHashHistory(),
  routes,
});
