export type RuntimeMode = "desktop" | "headless-api" | "windows-service" | "linux-service" | "cli" | "worker";
export type SidebarDefaultState = "expanded" | "collapsed";
export type MenuBehavior = "single-open" | "multi-open" | "free";
export type SubmenuVisualMode = "default" | "tree" | "compact" | "indented";
export type DatabaseDriver = "sqlite" | "mysql" | "postgres";
export type WorkspaceTabsMode = "enabled" | "disabled";

export const projectConfig = {
  app: {
    name: "Manifestador NF-e",
    shortName: "Manifestador",
    productName: "Manifestador NF-e",
    windowTitle: "Manifestador NF-e",
    subtitle: "distribuição fiscal • manifestação • XML • Tauri/Rust",
    description: "Aplicação nova para consultar, manifestar, baixar, importar, organizar e enviar XMLs de NF-e destinadas.",
    version: "1.0.0",
    mode: "desktop" as RuntimeMode,
    identifier: "br.com.wwsoftwares.manifestador-nfe",
    developer: "Wallace Kleiton – WWSoftware's",
    localDataDir: "manifestador-nfe",
    storagePrefix: "manifestador-nfe",
    supportUrl: "",
    documentationUrl: "",
  },
  features: {
    licensing: false,
    about: true,
    userGuide: true,
    logs: true,
    systemSettings: true,
    genericEntities: true,
    technicalSheet: true,
    sync: true,
    internalApi: true,
    scalarDocs: true,
    webhookService: true,
    websocketService: true,
    databaseSettings: true,
    integrations: true,
    tray: true,
    windowsService: true,
    linuxService: true,
    autoStartWithWindows: false,
    headlessMode: true,
    printPreview: true,
    manifestador: true,
  },
  defaultAdmin: {
    enabled: true,
    username: "admin",
    bootstrapStrategy: "internal" as const,
    bootstrapPasswordEnv: "MANIFESTADOR_NFE_BOOTSTRAP_ADMIN_PASSWORD",
    forcePasswordChangeOnFirstLogin: true,
  },
  database: {
    driver: "sqlite" as DatabaseDriver,
    sqlite: { path: "manifestador-nfe.db" },
    mysql: { host: "127.0.0.1", port: 3306, database: "manifestador_nfe", username: "root", password: "" },
    postgres: { host: "127.0.0.1", port: 5432, database: "manifestador_nfe", username: "postgres", password: "" },
    firebird: {
      supported: false,
      scope: "out-of-scope",
      note: "Firebird ignorado por compatibilidade nesta etapa.",
    },
  },
  dashboard: {
    enabled: true,
    demoMode: false,
    showSystemCards: true,
    showBusinessCards: true,
    showIntegrationCards: true,
    showCharts: true,
    blocks: {
      systemHealth: true,
      userStats: true,
      companyStats: true,
      licensingStatus: false,
      internalApiStatus: true,
      integrationStatus: true,
      syncStatus: true,
      financialSummary: false,
      customBusinessCards: false,
    },
  },
  api: {
    enabled: true,
    autoStart: false,
    restartOnConfigChange: true,
    host: "127.0.0.1",
    port: 61001,
    baseUrl: "http://127.0.0.1:61001",
    scalarUrl: "http://127.0.0.1:61001/docs",
    docsPath: "/docs",
    timeoutMs: 8000,
    logMode: "normal",
    openScalarAfterStart: false,
    docs: true,
    docsProvider: "scalar" as const,
    security: {
      bindHost: "127.0.0.1",
      allowPublicNetwork: false,
      requireToken: false,
      tokenHeader: "X-App-Token",
      corsEnabled: false,
      rateLimitEnabled: false,
      docsPublic: false,
      docsPublicLocal: true,
    },
  },
  sidebar: {
    defaultState: "expanded" as SidebarDefaultState,
    allowCollapse: true,
    menuBehavior: "single-open" as MenuBehavior,
    keepActiveParentExpanded: true,
    collapsePreviousOnNewSelection: true,
    persistUserMenuState: false,
    showSubmenuTreeLine: true,
    submenuVisualMode: "tree" as SubmenuVisualMode,
  },
  workspace: {
    tabsMode: "enabled" as WorkspaceTabsMode,
    persistTabs: true,
    maxTabs: 12,
    mobileTabs: false,
    mobileBreakpoint: 1180,
  },
  mobile: {
    enabled: true,
    frontend: "local-bundled" as const,
    localDatabase: "sqlite" as const,
    defaultMode: "unconfigured" as "unconfigured" | "local" | "remote",
    localAuthenticationOptional: true,
    localAuthenticationRequiredByDefault: false,
    standaloneSupported: true,
    restClientSupported: true,
    remoteWebView: false,
    remoteApiUserScope: true,
    remoteApiCompanyScope: true,
    synchronization: "product-defined" as const,
  },
  services: {
    webhook: {
      enabled: false,
      host: "0.0.0.0",
      port: 61003,
      basePath: "/webhooks",
      tokenRequired: true,
      tokenHeader: "X-Webhook-Token",
    },
    websocket: {
      enabled: false,
      host: "0.0.0.0",
      port: 61004,
      path: "/ws",
      tokenRequired: true,
      tokenQuery: "token",
      tokenHeader: "X-WebSocket-Token",
    },
  },
  tray: {
    enabled: true,
    minimizeToTray: true,
    closeToTray: false,
    alwaysUseTray: false,
    askBeforeExit: true,
    showServiceControls: true,
    showInternalApiStatus: true,
  },
  startup: {
    enabled: false,
    mode: "disabled" as "disabled" | "user-login" | "machine-startup",
  },
  integrations: {
    enabled: true,
    allowExternalApis: true,
    allowWebhooks: true,
    allowTokens: true,
    allowRequestLogs: true,
    allowRetryQueue: true,
  },
};

export const appFeatures = projectConfig.features;
export const sidebarConfig = projectConfig.sidebar;
export const workspaceConfig = projectConfig.workspace;
export const dashboardConfig = projectConfig.dashboard;
export const internalApiConfig = projectConfig.api;
export const serviceConfig = projectConfig.services;
export const databaseConfig = projectConfig.database;

export const trayConfig = projectConfig.tray;
export const startupConfig = projectConfig.startup;
export const integrationsConfig = projectConfig.integrations;
