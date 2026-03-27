import { describe, it, expect, beforeEach, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { setActivePinia, createPinia } from "pinia";
import SettingsModal from "../components/SettingsModal.vue";
import { useUiStore } from "../stores/ui";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue([]),
}));

describe("SettingsModal", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    localStorage.clear();
  });

  it("does not render when settings is closed", () => {
    const wrapper = mount(SettingsModal, { global: { stubs: { teleport: true } } });
    expect(wrapper.find(".modal-overlay").exists()).toBe(false);
  });

  it("renders when settings is open", () => {
    const uiStore = useUiStore();
    uiStore.toggleSettings();
    const wrapper = mount(SettingsModal, { global: { stubs: { teleport: true } } });
    expect(wrapper.find(".modal-overlay").exists()).toBe(true);
    expect(wrapper.find(".modal-header h2").text()).toBe("Settings");
  });

  it("closes when close button is clicked", async () => {
    const uiStore = useUiStore();
    uiStore.toggleSettings();
    const wrapper = mount(SettingsModal, { global: { stubs: { teleport: true } } });
    await wrapper.find(".close-btn").trigger("click");
    expect(uiStore.isSettingsOpen).toBe(false);
  });

  it("closes when overlay is clicked", async () => {
    const uiStore = useUiStore();
    uiStore.toggleSettings();
    const wrapper = mount(SettingsModal, { global: { stubs: { teleport: true } } });
    await wrapper.find(".modal-overlay").trigger("click");
    expect(uiStore.isSettingsOpen).toBe(false);
  });

  it("renders tab navigation", () => {
    const uiStore = useUiStore();
    uiStore.toggleSettings();
    const wrapper = mount(SettingsModal, { global: { stubs: { teleport: true } } });
    const tabs = wrapper.findAll(".tab-btn");
    expect(tabs.length).toBe(6);
  });

  it("has AI Configuration as the default active tab", () => {
    const uiStore = useUiStore();
    uiStore.toggleSettings();
    const wrapper = mount(SettingsModal, { global: { stubs: { teleport: true } } });
    const activeTab = wrapper.find(".tab-btn.active");
    expect(activeTab.text()).toContain("AI Configuration");
  });

  it("switches tabs when clicked", async () => {
    const uiStore = useUiStore();
    uiStore.toggleSettings();
    const wrapper = mount(SettingsModal, { global: { stubs: { teleport: true } } });
    const tabs = wrapper.findAll(".tab-btn");

    await tabs[1].trigger("click");
    await wrapper.vm.$nextTick();
    const tabsAfterFirst = wrapper.findAll(".tab-btn");
    expect(tabsAfterFirst[1].classes()).toContain("active");

    await tabs[3].trigger("click");
    await wrapper.vm.$nextTick();
    const tabsAfterSecond = wrapper.findAll(".tab-btn");
    expect(tabsAfterSecond[3].classes()).toContain("active");
  });
});
