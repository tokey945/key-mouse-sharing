import { createMemoryHistory, createRouter } from "vue-router";

import FileSharing from "@/pages/FileSharing.vue";
import KeyMouseSharing from "@/pages/KeyMouseSharing.vue";
import ScreenSharing from "@/pages/ScreenSharing.vue";

const routes = [
  { path: "/", component: KeyMouseSharing },
  { path: "/file/sharing", component: FileSharing },
  { path: "/screen/sharing", component: ScreenSharing },
];

const router = createRouter({
  history: createMemoryHistory(),
  routes,
});
export default router;
