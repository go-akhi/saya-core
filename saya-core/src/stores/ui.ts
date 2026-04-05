import { defineStore } from "pinia";
import { ref } from "vue";

export interface NotificationError {
  id: string;
  title?: string;
  message: string;
  type: "error" | "warning" | "info";
  pluginName?: string;
}

export const useUiStore = defineStore("ui", () => {
  const isSettingsOpen = ref(false);
  const isActionsBarVisible = ref(false);
  const notifications = ref<NotificationError[]>([]);

  function toggleSettings() {
    isSettingsOpen.value = !isSettingsOpen.value;
  }

  function showActionsBar() {
    isActionsBarVisible.value = true;
  }

  function hideActionsBar() {
    isActionsBarVisible.value = false;
  }

  function showError(error: NotificationError) {
    notifications.value.push(error);
  }

  function dismissError(id: string) {
    notifications.value = notifications.value.filter(n => n.id !== id);
  }

  return {
    isSettingsOpen,
    isActionsBarVisible,
    notifications,
    toggleSettings,
    showActionsBar,
    hideActionsBar,
    showError,
    dismissError,
  };
});
