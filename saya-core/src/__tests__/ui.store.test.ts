import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { useUiStore } from "../stores/ui";

describe("useUiStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("starts with settings closed", () => {
    const store = useUiStore();
    expect(store.isSettingsOpen).toBe(false);
  });

  it("starts with actions bar hidden", () => {
    const store = useUiStore();
    expect(store.isActionsBarVisible).toBe(false);
  });

  it("toggles settings open", () => {
    const store = useUiStore();
    store.toggleSettings();
    expect(store.isSettingsOpen).toBe(true);
  });

  it("toggles settings closed", () => {
    const store = useUiStore();
    store.toggleSettings();
    store.toggleSettings();
    expect(store.isSettingsOpen).toBe(false);
  });

  it("shows actions bar", () => {
    const store = useUiStore();
    store.showActionsBar();
    expect(store.isActionsBarVisible).toBe(true);
  });

  it("hides actions bar", () => {
    const store = useUiStore();
    store.showActionsBar();
    store.hideActionsBar();
    expect(store.isActionsBarVisible).toBe(false);
  });
});
