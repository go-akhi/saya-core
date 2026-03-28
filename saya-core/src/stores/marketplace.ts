import { defineStore } from "pinia";
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

export interface PluginColumnEntry {
  name: string;
  display: string;
  type: string;
  dtype: string;
  sortable: boolean;
}

export interface PluginEntryManifest {
  columns: PluginColumnEntry[];
}

export interface MarketplacePlugin {
  name: string;
  display_name: string;
  icon: string;
  version: string;
  description: string;
  repo_url: string;
  verified: boolean;
  manifest: PluginEntryManifest;
}

export interface RegistryVerifyResult {
  valid: boolean;
  plugins: MarketplacePlugin[];
  error: string | null;
}

const REGISTRY_URL = import.meta.env.VITE_REGISTRY_URL ?? "https://saya-org.github.io/saya/plugins.json";
const SKIP_VERIFICATION = import.meta.env.VITE_SKIP_SIGNATURE_VERIFICATION === "true";

export const useMarketplaceStore = defineStore("marketplace", () => {
  const plugins = ref<MarketplacePlugin[]>([]);
  const isLoading = ref(false);
  const error = ref<string | null>(null);
  const isVerified = ref(false);
  const readmeCache = ref<Record<string, string>>({});

  const sortedPlugins = computed(() => {
    return [...plugins.value].sort((a, b) => {
      if (a.verified !== b.verified) {
        return a.verified ? -1 : 1;
      }
      return a.display_name.localeCompare(b.display_name);
    });
  });

  const verifiedPlugins = computed(() => sortedPlugins.value.filter(p => p.verified));
  const communityPlugins = computed(() => sortedPlugins.value.filter(p => !p.verified));

  async function fetchRegistry(forceUrl?: string): Promise<void> {
    isLoading.value = true;
    error.value = null;
    plugins.value = [];

    try {
      const url = forceUrl ?? REGISTRY_URL;
      const json = await invoke<string>("fetch_plugin_registry", { url });

      if (SKIP_VERIFICATION) {
        const data = JSON.parse(json);
        plugins.value = data.plugins || [];
        isVerified.value = false;
      } else {
        const result = await invoke<RegistryVerifyResult>("verify_registry", { json });

        if (!result.valid) {
          error.value = result.error ?? "Registry integrity check failed";
          return;
        }

        isVerified.value = true;
        plugins.value = result.plugins;
      }
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      isLoading.value = false;
    }
  }

  async function fetchReadme(owner: string, repo: string): Promise<string> {
    const key = `${owner}/${repo}`;
    if (readmeCache.value[key]) {
      return readmeCache.value[key];
    }

    try {
      const content = await invoke<string>("fetch_plugin_readme", { owner, repo });
      readmeCache.value[key] = content;
      return content;
    } catch (e) {
      return "# Error\n\nFailed to load README content.";
    }
  }

  async function installPlugin(repoUrl: string): Promise<boolean> {
    isLoading.value = true;
    error.value = null;

    try {
      await invoke<boolean>("install_plugin_from_repo", { repoUrl });
      return true;
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e);
      return false;
    } finally {
      isLoading.value = false;
    }
  }

  function isInstalled(pluginName: string, installedPlugins: string[]): boolean {
    return installedPlugins.includes(pluginName);
  }

  return {
    plugins,
    sortedPlugins,
    verifiedPlugins,
    communityPlugins,
    isLoading,
    error,
    isVerified,
    fetchRegistry,
    fetchReadme,
    installPlugin,
    isInstalled,
  };
});
