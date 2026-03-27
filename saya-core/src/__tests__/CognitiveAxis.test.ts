import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { setActivePinia, createPinia } from "pinia";
import CognitiveAxis from "../components/CognitiveAxis.vue";
import { useAxesStore } from "../stores/axes";

describe("CognitiveAxis", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("renders 4 axis buttons", () => {
    const wrapper = mount(CognitiveAxis);
    const buttons = wrapper.findAll(".axis-btn");
    expect(buttons).toHaveLength(4);
  });

  it("renders correct axis labels", () => {
    const wrapper = mount(CognitiveAxis);
    const labels = wrapper.findAll(".axis-label").map((l) => l.text());
    expect(labels).toEqual(["Require", "Review", "Retain", "Relieve"]);
  });

  it("hides badge when count is 0", () => {
    const wrapper = mount(CognitiveAxis);
    const badges = wrapper.findAll(".axis-badge");
    expect(badges).toHaveLength(0);
  });

  it("shows badge counts from store when > 0", () => {
    const axesStore = useAxesStore();
    axesStore.setBadgeCounts({ Require: 5, Review: 3, Retain: 1, Relieve: 0 });
    const wrapper = mount(CognitiveAxis);
    const badges = wrapper.findAll(".axis-badge").map((b) => b.text());
    expect(badges).toEqual(["5", "3", "1"]);
  });

  it("clicking a button sets active cognitive axis", async () => {
    const axesStore = useAxesStore();
    const wrapper = mount(CognitiveAxis);
    const buttons = wrapper.findAll(".axis-btn");
    await buttons[1].trigger("click");
    expect(axesStore.activeCognitiveAxis).toBe("Review");
  });

  it("clicking active button deselects (toggles off)", async () => {
    const axesStore = useAxesStore();
    axesStore.setActiveCognitiveAxis("Review");
    const wrapper = mount(CognitiveAxis);
    const buttons = wrapper.findAll(".axis-btn");
    await buttons[1].trigger("click");
    expect(axesStore.activeCognitiveAxis).toBeNull();
  });

  it("active button gets active class", () => {
    const axesStore = useAxesStore();
    axesStore.setActiveCognitiveAxis("Retain");
    const wrapper = mount(CognitiveAxis);
    const buttons = wrapper.findAll(".axis-btn");
    expect(buttons[0].classes()).not.toContain("active");
    expect(buttons[2].classes()).toContain("active");
  });

  it("active button has inline background color", () => {
    const axesStore = useAxesStore();
    axesStore.setActiveCognitiveAxis("Require");
    const wrapper = mount(CognitiveAxis);
    const buttons = wrapper.findAll(".axis-btn");
    expect(buttons[0].attributes("style")).toContain("background-color");
  });

  it("uses full-height layout via flex-1", () => {
    const wrapper = mount(CognitiveAxis);
    const aside = wrapper.find(".cognitive-axis");
    expect(aside.classes()).toContain("cognitive-axis");
    const btn = wrapper.find(".axis-btn");
    expect(btn.classes()).toContain("axis-btn");
  });
});
