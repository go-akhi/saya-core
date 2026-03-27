import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface ContextAxis {
  id: number;
  name: string;
  description: string | null;
  icon: string | null;
  color: string | null;
  is_default: boolean;
}

export type CognitiveAxis = "Require" | "Review" | "Retain" | "Relieve" | null;

export interface CognitiveAxisDef {
  label: CognitiveAxis;
  color: string;
}

export const COGNITIVE_AXES: CognitiveAxisDef[] = [
  { label: "Require", color: "#DC5F3F" },
  { label: "Review", color: "#D97706" },
  { label: "Retain", color: "#706F6C" },
  { label: "Relieve", color: "#A8A7A3" },
];

export const useAxesStore = defineStore("axes", () => {
  const contextAxes = ref<ContextAxis[]>([]);
  const activeContextAxis = ref<number | null>(null);
  const activeCognitiveAxis = ref<CognitiveAxis>(null);
  const badgeCounts = ref<Record<string, number>>({
    Require: 0,
    Review: 0,
    Retain: 0,
    Relieve: 0,
  });

  const totalCount = computed(() =>
    Object.values(badgeCounts.value).reduce((a, b) => a + b, 0)
  );

  function setContextAxes(axes: ContextAxis[]) {
    contextAxes.value = axes;
    if (axes.length > 0 && activeContextAxis.value === null) {
      activeContextAxis.value = axes[0].id;
    }
  }

  function addContextAxis(axis: ContextAxis) {
    contextAxes.value.push(axis);
  }

  function setActiveContextAxis(id: number | null) {
    activeContextAxis.value = id;
  }

  function setActiveCognitiveAxis(axis: CognitiveAxis) {
    activeCognitiveAxis.value = axis;
  }

  function setBadgeCounts(counts: Record<string, number>) {
    badgeCounts.value = counts;
  }

  async function loadAxes() {
    try {
      contextAxes.value = await invoke<ContextAxis[]>("get_context_axes");
      if (contextAxes.value.length > 0 && activeContextAxis.value === null) {
        activeContextAxis.value = contextAxes.value[0].id;
      }
    } catch (e) {
      console.error("Failed to load context axes:", e);
    }
  }

  async function createAxis(data: { name: string; description?: string; icon?: string; color?: string }) {
    try {
      const result = await invoke<ContextAxis>("create_context_axis", {
        name: data.name,
        description: data.description || null,
        icon: data.icon || null,
        color: data.color || null,
      });
      contextAxes.value.push(result);
      return result;
    } catch (e) {
      throw e;
    }
  }

  async function updateAxis(id: number, data: { name?: string; description?: string; icon?: string; color?: string }) {
    try {
      await invoke("update_context_axis", {
        id,
        name: data.name ?? null,
        description: data.description ?? null,
        icon: data.icon ?? null,
        color: data.color ?? null,
      });
      const idx = contextAxes.value.findIndex((a) => a.id === id);
      if (idx !== -1) {
        contextAxes.value[idx] = { ...contextAxes.value[idx], ...data };
      }
    } catch (e) {
      throw e;
    }
  }

  async function deleteAxis(id: number) {
    try {
      await invoke("delete_context_axis", { id });
      contextAxes.value = contextAxes.value.filter((a) => a.id !== id);
      if (activeContextAxis.value === id && contextAxes.value.length > 0) {
        activeContextAxis.value = contextAxes.value[0].id;
      }
    } catch (e) {
      throw e;
    }
  }

  return {
    contextAxes,
    activeContextAxis,
    activeCognitiveAxis,
    badgeCounts,
    totalCount,
    setContextAxes,
    addContextAxis,
    setActiveContextAxis,
    setActiveCognitiveAxis,
    setBadgeCounts,
    loadAxes,
    createAxis,
    updateAxis,
    deleteAxis,
  };
});
