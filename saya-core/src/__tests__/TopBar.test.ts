import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { setActivePinia, createPinia } from "pinia";
import TopBar from "../components/TopBar.vue";
import { usePluginStore } from "../stores/plugins";
import { useAxesStore } from "../stores/axes";
import { useUiStore } from "../stores/ui";

describe("TopBar", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("renders the batch AI button", () => {
    const wrapper = mount(TopBar);
    expect(wrapper.find(".batch-ai-btn").exists()).toBe(true);
  });

  it("renders help and settings buttons", () => {
    const wrapper = mount(TopBar);
    const buttons = wrapper.findAll(".top-bar-right .icon-btn");
    expect(buttons).toHaveLength(2);
  });

  it("renders empty plugin picker when no plugins", () => {
    const wrapper = mount(TopBar);
    expect(wrapper.find(".plugin-btn.empty").exists()).toBe(true);
  });

  it("renders plugin buttons from store", () => {
    const pluginStore = usePluginStore();
    pluginStore.plugins = [
      { name: "email", display_name: "Email", icon: "📧", version: "0.1.0", is_enabled: true },
      { name: "tasks", display_name: "Tasks", icon: "✅", version: "0.1.0", is_enabled: true },
    ];
    const wrapper = mount(TopBar);
    const pluginBtns = wrapper.findAll(".plugin-btn:not(.empty)");
    expect(pluginBtns).toHaveLength(2);
  });

  it("renders context axis tabs", () => {
    const axesStore = useAxesStore();
    axesStore.setContextAxes([
      { id: 1, name: "Work", description: null, icon: "💼", color: "#3B82F6", is_default: true },
      { id: 2, name: "Personal", description: null, icon: "🏠", color: "#10B981", is_default: true },
    ]);
    const wrapper = mount(TopBar);
    const tabs = wrapper.findAll(".filter-tab");
    expect(tabs).toHaveLength(2);
    expect(tabs[0].text()).toContain("Work");
    expect(tabs[1].text()).toContain("Personal");
  });

  it("clicking settings button toggles settings", async () => {
    const uiStore = useUiStore();
    const wrapper = mount(TopBar);
    const settingsBtn = wrapper.findAll(".top-bar-right .icon-btn")[1];
    await settingsBtn.trigger("click");
    expect(uiStore.isSettingsOpen).toBe(true);
  });

  it("clicking plugin button sets active plugin", async () => {
    const pluginStore = usePluginStore();
    pluginStore.plugins = [
      { name: "email", display_name: "Email", icon: "📧", version: "0.1.0", is_enabled: true },
    ];
    const wrapper = mount(TopBar);
    await wrapper.find(".plugin-btn").trigger("click");
    expect(pluginStore.activePlugin).toBe("email");
  });
});
