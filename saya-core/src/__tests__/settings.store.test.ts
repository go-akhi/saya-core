import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { useSettingsStore } from "../stores/settings";

describe("settings store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
  });

  it("defaults to system theme", () => {
    const store = useSettingsStore();
    expect(store.theme).toBe("system");
  });

  it("defaults cognitive axis to Require", () => {
    const store = useSettingsStore();
    expect(store.defaultCognitiveAxis).toBe("Require");
  });

  it("reads theme from localStorage", () => {
    localStorage.setItem("saya-theme", "dark");
    const store = useSettingsStore();
    expect(store.theme).toBe("dark");
  });

  it("setTheme persists to localStorage", () => {
    const store = useSettingsStore();
    store.setTheme("light");
    expect(localStorage.getItem("saya-theme")).toBe("light");
  });

  it("setTheme applies data-theme attribute for dark", () => {
    const store = useSettingsStore();
    store.setTheme("dark");
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  });

  it("setTheme removes data-theme attribute for light", () => {
    document.documentElement.setAttribute("data-theme", "dark");
    const store = useSettingsStore();
    store.setTheme("light");
    expect(document.documentElement.hasAttribute("data-theme")).toBe(false);
  });

  it("setDefaultCognitiveAxis persists to localStorage", () => {
    const store = useSettingsStore();
    store.setDefaultCognitiveAxis("Review");
    expect(localStorage.getItem("saya-default-cognitive-axis")).toBe("Review");
  });

  it("initTheme applies current theme", () => {
    localStorage.setItem("saya-theme", "dark");
    const store = useSettingsStore();
    store.initTheme();
    expect(document.documentElement.getAttribute("data-theme")).toBe("dark");
  });
});
