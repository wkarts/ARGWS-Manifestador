import { storageKey } from "../../config/appBranding";

export type MobileOperationMode = "unconfigured" | "local" | "remote";

export interface MobileConnectionConfig {
  mode: MobileOperationMode;
  localAuthenticationRequired: boolean;
  connectionName: string;
  baseUrl: string;
  tokenHeader: string;
}

const CONFIG_KEY = storageKey("mobile-connection");
const GATEWAY_TOKEN_KEY = storageKey("mobile-gateway-token");

const DEFAULT_CONFIG: MobileConnectionConfig = {
  mode: "unconfigured",
  localAuthenticationRequired: false,
  connectionName: "Servidor principal",
  baseUrl: "",
  tokenHeader: "X-App-Token",
};

function localStore(): Storage | null {
  return typeof window === "undefined" ? null : window.localStorage;
}

function sessionStore(): Storage | null {
  return typeof window === "undefined" ? null : window.sessionStorage;
}

function normalizeBaseUrl(value: string): string {
  return value.trim().replace(/\/+$/, "");
}

export function loadMobileConnection(): MobileConnectionConfig {
  const raw = localStore()?.getItem(CONFIG_KEY);
  if (!raw) return { ...DEFAULT_CONFIG };
  try {
    const parsed = JSON.parse(raw) as Partial<MobileConnectionConfig>;
    return {
      mode: parsed.mode === "local" || parsed.mode === "remote" ? parsed.mode : "unconfigured",
      localAuthenticationRequired: Boolean(parsed.localAuthenticationRequired),
      connectionName: parsed.connectionName?.trim() || DEFAULT_CONFIG.connectionName,
      baseUrl: normalizeBaseUrl(parsed.baseUrl || ""),
      tokenHeader: parsed.tokenHeader?.trim() || DEFAULT_CONFIG.tokenHeader,
    };
  } catch {
    return { ...DEFAULT_CONFIG };
  }
}

export function saveMobileConnection(
  config: MobileConnectionConfig,
  gatewayToken?: string,
): MobileConnectionConfig {
  const normalized: MobileConnectionConfig = {
    mode: config.mode,
    localAuthenticationRequired: Boolean(config.localAuthenticationRequired),
    connectionName: config.connectionName.trim() || DEFAULT_CONFIG.connectionName,
    baseUrl: normalizeBaseUrl(config.baseUrl),
    tokenHeader: config.tokenHeader.trim() || DEFAULT_CONFIG.tokenHeader,
  };
  localStore()?.setItem(CONFIG_KEY, JSON.stringify(normalized));
  setMobileGatewayToken(gatewayToken || "");
  return normalized;
}

export function resetMobileConnection(): void {
  localStore()?.removeItem(CONFIG_KEY);
  sessionStore()?.removeItem(GATEWAY_TOKEN_KEY);
}

export function mobileMode(): MobileOperationMode {
  return loadMobileConnection().mode;
}

export function isMobileRemoteMode(): boolean {
  return mobileMode() === "remote";
}

export function mobileLocalAuthenticationRequired(): boolean {
  const config = loadMobileConnection();
  return config.mode === "local" && config.localAuthenticationRequired;
}

export function setMobileGatewayToken(value: string): void {
  const store = sessionStore();
  if (!store) return;
  const normalized = value.trim();
  if (!normalized) {
    store.removeItem(GATEWAY_TOKEN_KEY);
  } else {
    store.setItem(GATEWAY_TOKEN_KEY, normalized);
  }
}

export function mobileGatewayToken(): string {
  return sessionStore()?.getItem(GATEWAY_TOKEN_KEY) || "";
}

export async function testMobileConnection(
  config: MobileConnectionConfig,
  gatewayToken = "",
): Promise<{ ok: boolean; message: string }> {
  const baseUrl = normalizeBaseUrl(config.baseUrl);
  if (!baseUrl) return { ok: false, message: "Informe a URL da API." };
  const headers: Record<string, string> = { Accept: "application/json" };
  if (gatewayToken.trim()) headers[config.tokenHeader.trim() || "X-App-Token"] = gatewayToken.trim();
  try {
    const response = await fetch(`${baseUrl}/health`, { headers });
    if (!response.ok) {
      return { ok: false, message: `Servidor respondeu HTTP ${response.status}.` };
    }
    const payload = await response.json() as { ok?: boolean; database?: boolean };
    return {
      ok: payload.ok !== false,
      message: payload.database === false
        ? "A API respondeu, mas o banco está indisponível."
        : "Conexão com a API validada.",
    };
  } catch (error) {
    return {
      ok: false,
      message: error instanceof Error ? error.message : "Falha ao conectar na API.",
    };
  }
}
