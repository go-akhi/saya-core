import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { usePluginStore } from "../stores/plugins";

describe("usePluginStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("starts with empty plugins list", () => {
    const store = usePluginStore();
    expect(store.plugins).toEqual([]);
  });

  it("starts with no active plugin", () => {
    const store = usePluginStore();
    expect(store.activePlugin).toBeNull();
  });

  it("sets active plugin", () => {
    const store = usePluginStore();
    store.setActivePlugin("email");
    expect(store.activePlugin).toBe("email");
  });

  it("clears active plugin with null", () => {
    const store = usePluginStore();
    store.setActivePlugin("email");
    store.setActivePlugin(null);
    expect(store.activePlugin).toBeNull();
  });
});
