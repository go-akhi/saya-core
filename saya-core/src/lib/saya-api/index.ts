import type {
  Item,
  QueryOptions,
  MutationOptions,
  AiActionRequest,
  SubscriptionOptions,
  SayaMessage,
  ResponsePayload,
  PluginInfo,
  PluginManifest,
  PluginSettings,
  ErrorPayload,
  CompletionRequest,
  CompletionResponse,
} from "./types";

function generateId(): string {
  return `${Date.now()}-${Math.random().toString(36).substring(2, 9)}`;
}

export class SayaApi {
  private targetWindow: Window | null = null;
  private pendingRequests: Map<string, {
    resolve: (value: unknown) => void;
    reject: (reason: Error) => void;
    timeout: ReturnType<typeof setTimeout>;
  }> = new Map();
  private subscriptions: Map<string, (payload: unknown) => void> = new Map();
  private messageListener: ((event: MessageEvent) => void) | null = null;
  private pluginName: string;
  private _isConnected = false;

  get isConnected(): boolean {
    return this._isConnected;
  }

  constructor(pluginName: string) {
    this.pluginName = pluginName;
  }

  connect(target: Window): void {
    this.targetWindow = target;
    this.setupMessageListener();
    this._isConnected = true;
  }

  disconnect(): void {
    if (this.messageListener) {
      window.removeEventListener("message", this.messageListener);
      this.messageListener = null;
    }
    this.pendingRequests.forEach(({ timeout }) => clearTimeout(timeout));
    this.pendingRequests.clear();
    this.subscriptions.clear();
    this.targetWindow = null;
    this._isConnected = false;
  }

  private setupMessageListener(): void {
    this.messageListener = (event: MessageEvent) => {
      const message = event.data as SayaMessage;
      if (!message || typeof message !== "object") return;
      if (message.source === "core" && message.plugin === this.pluginName) {
        this.handleMessage(message);
      }
    };
    window.addEventListener("message", this.messageListener);
  }

  private handleMessage(message: SayaMessage): void {
    if (message.type === "response") {
      const pending = this.pendingRequests.get(message.id);
      if (pending) {
        clearTimeout(pending.timeout);
        this.pendingRequests.delete(message.id);
        const payload = message.payload as ResponsePayload;
        if (payload.success) {
          pending.resolve(payload.data);
        } else {
          pending.reject(new Error(payload.error || "Unknown error"));
        }
      }
    } else if (message.type === "event") {
      const callback = this.subscriptions.get(message.id);
      if (callback) {
        callback(message.payload);
      }
    }
  }

  private sendMessage<T = unknown>(message: Omit<SayaMessage, "id" | "source">, timeout = 30000): Promise<T> {
    return new Promise((resolve, reject) => {
      if (!this.targetWindow) {
        reject(new Error("Not connected to core"));
        return;
      }

      const id = generateId();
      const fullMessage: SayaMessage = {
        ...message,
        id,
        source: "plugin",
      };

      const timeoutHandle = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(new Error(`Request ${id} timed out after ${timeout}ms`));
      }, timeout);

      this.pendingRequests.set(id, {
        resolve: resolve as (value: unknown) => void,
        reject,
        timeout: timeoutHandle,
      });

      this.targetWindow.postMessage(fullMessage, "*");
    });
  }

  async query<T extends Item = Item>(options: QueryOptions): Promise<T[]> {
    return this.sendMessage<T[]>({
      type: "query",
      payload: options,
      plugin: this.pluginName,
    });
  }

  async mutate<T extends Item = Item>(options: MutationOptions): Promise<T> {
    return this.sendMessage<T>({
      type: "mutate",
      payload: options,
      plugin: this.pluginName,
    });
  }

  async aiAction(options: AiActionRequest): Promise<Record<string, unknown>> {
    return this.sendMessage<Record<string, unknown>>({
      type: "ai_action",
      payload: options,
      plugin: this.pluginName,
    });
  }

  subscribe(event: SubscriptionOptions["event"], callback: (payload: unknown) => void): string {
    const id = generateId();
    this.subscriptions.set(id, callback);

    this.sendMessage({
      type: "subscribe",
      payload: { plugin: this.pluginName, event },
      plugin: this.pluginName,
    }).catch((err) => {
      this.subscriptions.delete(id);
      console.error("Subscribe error:", err);
    });

    return id;
  }

  unsubscribe(subscriptionId: string): void {
    const callback = this.subscriptions.get(subscriptionId);
    if (callback) {
      this.subscriptions.delete(subscriptionId);
      this.sendMessage({
        type: "unsubscribe",
        payload: { subscriptionId },
        plugin: this.pluginName,
      }).catch(() => {});
    }
  }

  async getManifest(): Promise<PluginManifest> {
    return this.sendMessage<PluginManifest>({
      type: "query",
      payload: { plugin: this.pluginName, operation: "get_manifest" },
      plugin: this.pluginName,
    });
  }

  async getPluginInfo(): Promise<PluginInfo> {
    return this.sendMessage<PluginInfo>({
      type: "query",
      payload: { plugin: this.pluginName, operation: "get_info" },
      plugin: this.pluginName,
    });
  }

  async saveSettings(settings: PluginSettings): Promise<void> {
    await this.sendMessage<void>({
      type: "mutate",
      payload: { plugin: this.pluginName, operation: "save_settings", data: settings },
      plugin: this.pluginName,
    });
  }

  async loadSettings(): Promise<PluginSettings> {
    return this.sendMessage<PluginSettings>({
      type: "query",
      payload: { plugin: this.pluginName, operation: "load_settings" },
      plugin: this.pluginName,
    });
  }

  async showError(payload: ErrorPayload): Promise<void> {
    return this.sendMessage<void>({
      type: "show_error",
      payload,
      plugin: this.pluginName,
    });
  }

  async complete(request: CompletionRequest): Promise<CompletionResponse> {
    return this.sendMessage<CompletionResponse>({
      type: "complete",
      payload: request,
      plugin: this.pluginName,
    });
  }
}

export function createSayaApi(pluginName: string): SayaApi {
  return new SayaApi(pluginName);
}
