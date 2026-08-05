import "./assets/css/main.css";
import { createApp } from "vue";
import { createPinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";
import ui from "@nuxt/ui/vue-plugin";
import App from "./App.vue";

// Nuxt UI requires vue-router to be present even though this app has no
// real navigation — we just give it an empty route table.
const router = createRouter({
  routes: [],
  history: createWebHistory(),
});

createApp(App).use(createPinia()).use(router).use(ui).mount("#app");
