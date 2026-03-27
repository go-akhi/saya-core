import { describe, it, expect, beforeEach } from "vitest";
import { setActivePinia, createPinia } from "pinia";
import { useAxesStore, COGNITIVE_AXES } from "../stores/axes";

describe("useAxesStore", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("starts with empty context axes", () => {
    const store = useAxesStore();
    expect(store.contextAxes).toEqual([]);
  });

  it("starts with no active cognitive axis", () => {
    const store = useAxesStore();
    expect(store.activeCognitiveAxis).toBeNull();
  });

  it("starts with zero badge counts", () => {
    const store = useAxesStore();
    expect(store.badgeCounts).toEqual({
      Require: 0,
      Review: 0,
      Retain: 0,
      Relieve: 0,
    });
  });

  it("computes total count as sum of badges", () => {
    const store = useAxesStore();
    store.setBadgeCounts({ Require: 3, Review: 2, Retain: 1, Relieve: 0 });
    expect(store.totalCount).toBe(6);
  });

  it("auto-selects first context axis when set", () => {
    const store = useAxesStore();
    store.setContextAxes([
      { id: 1, name: "Work", description: null, icon: null, color: null, is_default: true },
      { id: 2, name: "Personal", description: null, icon: null, color: null, is_default: true },
    ]);
    expect(store.activeContextAxis).toBe(1);
  });

  it("does not override active context axis if already set", () => {
    const store = useAxesStore();
    store.setActiveContextAxis(2);
    store.setContextAxes([
      { id: 1, name: "Work", description: null, icon: null, color: null, is_default: true },
    ]);
    expect(store.activeContextAxis).toBe(2);
  });

  it("adds a new context axis", () => {
    const store = useAxesStore();
    store.addContextAxis({
      id: 10,
      name: "Side Project",
      description: "My hobby work",
      icon: null,
      color: null,
      is_default: false,
    });
    expect(store.contextAxes).toHaveLength(1);
    expect(store.contextAxes[0].name).toBe("Side Project");
    expect(store.contextAxes[0].description).toBe("My hobby work");
  });

  it("sets active cognitive axis", () => {
    const store = useAxesStore();
    store.setActiveCognitiveAxis("Require");
    expect(store.activeCognitiveAxis).toBe("Require");
  });

  it("clears active cognitive axis with null", () => {
    const store = useAxesStore();
    store.setActiveCognitiveAxis("Review");
    store.setActiveCognitiveAxis(null);
    expect(store.activeCognitiveAxis).toBeNull();
  });

  it("exports exactly 4 cognitive axes", () => {
    expect(COGNITIVE_AXES).toHaveLength(4);
    expect(COGNITIVE_AXES.map((a) => a.label)).toEqual([
      "Require",
      "Review",
      "Retain",
      "Relieve",
    ]);
  });

  it("each cognitive axis has a color", () => {
    for (const axis of COGNITIVE_AXES) {
      expect(axis.color).toMatch(/^#[0-9A-Fa-f]{6}$/);
    }
  });
});
