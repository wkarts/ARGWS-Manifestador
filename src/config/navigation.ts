import { isNativeMobileRuntime } from "../core/runtime/RuntimeProvider";
import { appFeatures } from "./projectConfig";
import { entityConfigs } from "./entities";

export type MenuSection = "dashboard" | "manifestador" | "cadastro" | "sistema" | "ferramentas" | "documentacao";

export interface MenuItemConfig {
  title: string;
  route: string;
  permission?: string;
  feature?: keyof typeof appFeatures;
  section: MenuSection;
  eyebrow?: string;
  description?: string;
  desktopOnly?: boolean;
}

export const menuItems: MenuItemConfig[] = [
  { title: "Visão fiscal", route: "/", permission: "dashboard:view", section: "dashboard" },

  { title: "Empresas", route: "/empresas", permission: "empresas:view", section: "cadastro" },
  { title: "Manifestação NF-e", route: "/manifestador", permission: "manifestador:view", feature: "manifestador", section: "manifestador", description: "Consultar, importar, manifestar, baixar e exportar NF-e destinadas." },
  { title: "SMTP global", route: "/sistema/smtp", permission: "config:view", section: "sistema", description: "Configuração de e-mail da aplicação." },
  ...Object.values(entityConfigs).map((entity) => ({
    title: entity.title,
    route: entity.route,
    permission: "cadastros:view",
    feature: "genericEntities" as keyof typeof appFeatures,
    section: "cadastro" as MenuSection,
  })),

  { title: "Usuários", route: "/usuarios", permission: "usuarios:view", section: "sistema" },
  { title: "Perfis de usuários", route: "/perfis", permission: "perfis:view", section: "sistema" },
  { title: "Logs", route: "/logs", permission: "config:view", feature: "logs", section: "sistema" },
  { title: "Configurações da aplicação", desktopOnly: true, route: "/sistema", permission: "config:view", feature: "systemSettings", section: "sistema" },
  { title: "Banco de dados", route: "/sistema/banco", permission: "config:view", feature: "databaseSettings", section: "sistema" },
  { title: "Licenciamento", route: "/licenciamento", permission: "config:view", feature: "licensing", section: "sistema" },
  { title: "Diagnósticos", desktopOnly: true, route: "/runtime", permission: "config:view", section: "sistema" },

  { title: "API Interna", desktopOnly: true, route: "/api-interna", permission: "config:view", feature: "internalApi", section: "sistema" },
  { title: "Documentação Scalar", desktopOnly: true, route: "/documentacao/scalar", permission: "config:view", feature: "scalarDocs", section: "sistema" },
  { title: "Webhooks", desktopOnly: true, route: "/webhooks", permission: "config:view", feature: "webhookService", section: "sistema" },
  { title: "WebSocket", desktopOnly: true, route: "/websocket", permission: "config:view", feature: "websocketService", section: "sistema" },
  { title: "Integrações", route: "/integracoes", permission: "config:view", feature: "integrations", section: "ferramentas" },
  { title: "Sincronização", route: "/sincronizacao", permission: "config:view", feature: "sync", section: "ferramentas" },
  { title: "Ficha técnica", route: "/ficha-tecnica", permission: "config:view", feature: "technicalSheet", section: "ferramentas" },

  { title: "Sobre", route: "/sobre", feature: "about", section: "documentacao" },
  { title: "Guia do usuário", route: "/documentacao/guia", feature: "userGuide", section: "documentacao" },
];

export function isFeatureEnabled(feature?: keyof typeof appFeatures): boolean {
  return feature ? appFeatures[feature] !== false : true;
}

export function visibleMenuItems(): MenuItemConfig[] {
  const nativeMobile = isNativeMobileRuntime();
  return menuItems.filter((item) => isFeatureEnabled(item.feature) && !(nativeMobile && item.desktopOnly));
}


export function findMenuItemByRoute(route: string): MenuItemConfig | undefined {
  const normalize = (value: string) => {
    const clean = (value || "/").replace(/\/+$/, "");
    return clean || "/";
  };

  const target = normalize(route);
  return visibleMenuItems().find((item) => normalize(item.route) === target);
}
