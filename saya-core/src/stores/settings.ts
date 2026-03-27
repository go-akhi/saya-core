import { defineStore } from "pinia";
import { ref } from "vue";
import type { CognitiveAxis } from "./axes";

export type Theme = "light" | "dark" | "system";

export const useSettingsStore = defineStore("settings", () => {
  const theme = ref<Theme>((localStorage.getItem("saya-theme") as Theme) || "system");
  const defaultCognitiveAxis = ref<CognitiveAxis>(
    (localStorage.getItem("saya-default-cognitive-axis") as CognitiveAxis) || "Require"
  );

  function applyTheme(t: Theme) {
    const root = document.documentElement;
    root.removeAttribute("data-theme");
    if (t === "dark") {
      root.setAttribute("data-theme", "dark");
    } else if (t === "system") {
      const prefersDark = window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches;
      if (prefersDark) {
        root.setAttribute("data-theme", "dark");
      }
    }
  }

  function setTheme(t: Theme) {
    theme.value = t;
    localStorage.setItem("saya-theme", t as string);
    applyTheme(t);
  }

  function setDefaultCognitiveAxis(axis: CognitiveAxis) {
    defaultCognitiveAxis.value = axis;
    if (axis) {
      localStorage.setItem("saya-default-cognitive-axis", axis);
    }
  }

  function initTheme() {
    applyTheme(theme.value);
  }

  return {
    theme,
    defaultCognitiveAxis,
    setTheme,
    setDefaultCognitiveAxis,
    initTheme,
  };
});
