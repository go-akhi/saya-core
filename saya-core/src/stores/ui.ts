import { defineStore } from "pinia";
import { ref } from "vue";

export const useUiStore = defineStore("ui", () => {
  const isSettingsOpen = ref(false);
  const isActionsBarVisible = ref(false);

  function toggleSettings() {
    isSettingsOpen.value = !isSettingsOpen.value;
  }

  function showActionsBar() {
    isActionsBarVisible.value = true;
  }

  function hideActionsBar() {
    isActionsBarVisible.value = false;
  }

  return {
    isSettingsOpen,
    isActionsBarVisible,
    toggleSettings,
    showActionsBar,
    hideActionsBar,
  };
});
