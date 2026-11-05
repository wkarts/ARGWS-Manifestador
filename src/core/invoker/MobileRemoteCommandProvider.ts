import { storageKey } from "../../config/appBranding";
import {
  loadMobileConnection,
  mobileGatewayToken,
} from "../mobile/MobileConnection";
import type { CommandArgs, CommandProvider } from "./CommandProvider";

interface ApiCommandResponse<T> {
  ok: boolean;
  result?: T;
  error?: string;
}

const SESSION_KEY = storageKey("session");
const ACTIVE_COMPANY_KEY = storageKey("active-company");

function runtimeSessionToken(): string {
  return typeof window === "undefined"
    ? ""
    : window.sessionStorage.getItem(SESSION_KEY) || "";
}

function activeCompanyId(): number | null {
  if (typeof window === "undefined") return null;
  const raw = window.localStorage.getItem(ACTIVE_COMPANY_KEY);
  const value = raw ? Number(raw) : NaN;
  return Number.isFinite(value) ? value : null;
}

function apiHeaders(): Record<string, string> {
  const config = loadMobileConnection();
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    Accept: "application/json",
  };
  const gatewayToken = mobileGatewayToken();
  if (gatewayToken) headers[config.tokenHeader || "X-App-Token"] = gatewayToken;
  return headers;
}

async function parseResponse<T>(response: Response, command: string): Promise<T> {
  const raw = await response.text();
  let payload: unknown;
  try {
    payload = raw ? JSON.parse(raw) : null;
  } catch {
    throw new Error(`Resposta inválida da API para ${command}: ${raw.slice(0, 180) || "vazia"}`);
  }
  if (!response.ok) {
    const message = typeof payload === "object" && payload && "error" in payload
      ? String((payload as { error?: unknown }).error || "")
      : "";
    throw new Error(message || `A API rejeitou ${command} com HTTP ${response.status}.`);
  }
  return payload as T;
}

export class MobileRemoteCommandProvider implements CommandProvider {
  providerName(): string {
    return "mobile-remote-api";
  }

  async invoke<T>(command: string, args?: CommandArgs): Promise<T> {
    const config = loadMobileConnection();
    if (config.mode !== "remote" || !config.baseUrl) {
      throw new Error("A conexão remota do aplicativo mobile não está configurada.");
    }

    const source = { ...(args || {}) } as Record<string, unknown>;
    const token = runtimeSessionToken();
    if (token && !source.session_token) source.session_token = token;
    const companyId = activeCompanyId();
    if (companyId != null && !source.active_company_id) source.active_company_id = companyId;

    const authRouteByCommand: Record<string, string> = {
      auth_login: "/api/auth/login",
      auth_restore: "/api/auth/restore",
      auth_logout: "/api/auth/logout",
      auth_change_password: "/api/auth/change-password",
      auth_password_policy: "/api/auth/password-policy",
    };
    const authRoute = authRouteByCommand[command];
    if (authRoute) {
      if (command === "auth_login") source.channel = "mobile";
      const policyRequest = command === "auth_password_policy";
      const response = await fetch(`${config.baseUrl}${authRoute}`, {
        method: policyRequest ? "GET" : "POST",
        headers: apiHeaders(),
        body: policyRequest ? undefined : JSON.stringify(source),
      });
      return parseResponse<T>(response, command);
    }

    const response = await fetch(`${config.baseUrl}/api/command`, {
      method: "POST",
      headers: apiHeaders(),
      body: JSON.stringify({ command, args: source }),
    });
    const payload = await parseResponse<ApiCommandResponse<T>>(response, command);
    if (!payload.ok) throw new Error(payload.error || `Falha ao executar ${command}.`);
    return payload.result as T;
  }
}
