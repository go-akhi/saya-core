import { describe, it, expect, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { setActivePinia, createPinia } from "pinia";
import AddContextAxis from "../components/AddContextAxis.vue";
import { useAxesStore } from "../stores/axes";

function mountComponent() {
  return mount(AddContextAxis, {
    global: { stubs: { teleport: true } },
  });
}

describe("AddContextAxis", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
  });

  it("renders the add button", () => {
    const wrapper = mountComponent();
    expect(wrapper.find(".add-axis-btn").exists()).toBe(true);
    expect(wrapper.find(".add-axis-btn").text()).toContain("+");
  });

  it("does not show modal initially", () => {
    const wrapper = mountComponent();
    expect(wrapper.find(".modal-overlay").exists()).toBe(false);
  });

  it("opens modal when add button is clicked", async () => {
    const wrapper = mountComponent();
    await wrapper.find(".add-axis-btn").trigger("click");
    expect(wrapper.find(".modal-overlay").exists()).toBe(true);
  });

  it("modal has name input and description input", async () => {
    const wrapper = mountComponent();
    await wrapper.find(".add-axis-btn").trigger("click");
    expect(wrapper.find('input[name="name"]').exists()).toBe(true);
    expect(wrapper.find('input[name="description"]').exists()).toBe(true);
  });

  it("modal has save and cancel buttons", async () => {
    const wrapper = mountComponent();
    await wrapper.find(".add-axis-btn").trigger("click");
    const buttons = wrapper.findAll(".modal-actions button");
    expect(buttons).toHaveLength(2);
  });

  it("closes modal on cancel", async () => {
    const wrapper = mountComponent();
    await wrapper.find(".add-axis-btn").trigger("click");
    const cancelBtn = wrapper.findAll(".modal-actions button")[0];
    await cancelBtn.trigger("click");
    expect(wrapper.find(".modal-overlay").exists()).toBe(false);
  });

  it("closes modal on overlay click", async () => {
    const wrapper = mountComponent();
    await wrapper.find(".add-axis-btn").trigger("click");
    await wrapper.find(".modal-overlay").trigger("click");
    expect(wrapper.find(".modal-overlay").exists()).toBe(false);
  });

  it("save is disabled when name is empty", async () => {
    const wrapper = mountComponent();
    await wrapper.find(".add-axis-btn").trigger("click");
    const saveBtn = wrapper.findAll(".modal-actions button")[1];
    expect(saveBtn.attributes("disabled")).toBeDefined();
  });

  it("save is enabled when name is filled", async () => {
    const wrapper = mountComponent();
    await wrapper.find(".add-axis-btn").trigger("click");
    await wrapper.find('input[name="name"]').setValue("Side Project");
    const saveBtn = wrapper.findAll(".modal-actions button")[1];
    expect(saveBtn.attributes("disabled")).toBeUndefined();
  });

  it("adds context axis to store on save", async () => {
    const axesStore = useAxesStore();
    const wrapper = mountComponent();
    await wrapper.find(".add-axis-btn").trigger("click");
    await wrapper.find('input[name="name"]').setValue("Side Project");
    await wrapper.find('input[name="description"]').setValue("My hobby");
    await wrapper.findAll(".modal-actions button")[1].trigger("click");

    expect(axesStore.contextAxes).toHaveLength(1);
    expect(axesStore.contextAxes[0].name).toBe("Side Project");
    expect(axesStore.contextAxes[0].description).toBe("My hobby");
    expect(axesStore.contextAxes[0].is_default).toBe(false);
  });

  it("clears inputs after save", async () => {
    const wrapper = mountComponent();
    await wrapper.find(".add-axis-btn").trigger("click");
    await wrapper.find('input[name="name"]').setValue("Test");
    await wrapper.findAll(".modal-actions button")[1].trigger("click");

    await wrapper.find(".add-axis-btn").trigger("click");
    const nameInput = wrapper.find('input[name="name"]');
    expect((nameInput.element as HTMLInputElement).value).toBe("");
  });
});
