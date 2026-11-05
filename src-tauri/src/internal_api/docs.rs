pub fn scalar_html(docs_path: &str, token: Option<&str>) -> String {
    let openapi_path = if docs_path.starts_with("/__internal_api") {
        "/__internal_api/openapi.json"
    } else {
        "/openapi.json"
    };
    let openapi_url = token
        .map(|value| format!("{}?token={}", openapi_path, value))
        .unwrap_or_else(|| openapi_path.to_string());
    format!(
        r#"<!doctype html>
<html lang="pt-BR">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>Manifestador NF-e API Docs</title>
  <style>
    :root {{ color-scheme: light only; }}
    html, body {{ margin:0; min-height:100%; background:#ffffff !important; color:#172033 !important; }}
    body {{ font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; }}
    scalar-api-reference {{
      --scalar-background-1:#ffffff !important;
      --scalar-background-2:#f8fafc !important;
      --scalar-background-3:#eef2f7 !important;
      --scalar-color-1:#172033 !important;
      --scalar-color-2:#475569 !important;
      --scalar-color-3:#64748b !important;
      color-scheme: light only !important;
    }}
  </style>
</head>
<body>
  <script id="api-reference" data-url="{openapi_url}" data-theme="default"></script>
  <script src="https://cdn.jsdelivr.net/npm/@scalar/api-reference"></script>
</body>
</html>"#
    )
}

pub fn openapi_json(token_header: &str, require_token: bool, docs_path: &str) -> serde_json::Value {
    let protected_security = if require_token {
        serde_json::json!([{ "LocalToken": [] }])
    } else {
        serde_json::json!([])
    };
    serde_json::json!({
        "openapi": "3.0.3",
        "info": {
            "title": "Manifestador NF-e Internal API",
            "version": env!("CARGO_PKG_VERSION"),
            "description": "API para desktop/headless/CloudPanel com sessão por usuário, perfil e empresa. O token global é uma camada opcional adicional."
        },
        "components": {
            "securitySchemes": {
                "LocalToken": {
                    "type": "apiKey",
                    "in": "header",
                    "name": token_header,
                    "description": "Token global opcional do gateway. Não substitui a sessão do usuário."
                }
            },
            "schemas": {
                "AuthLoginRequest": {
                    "type": "object",
                    "required": ["login", "senha"],
                    "properties": {
                        "login": { "type": "string" },
                        "senha": { "type": "string", "format": "password" },
                        "channel": {
                            "type": "string",
                            "enum": ["api", "mobile"],
                            "default": "api"
                        }
                    }
                },
                "SessionRequest": {
                    "type": "object",
                    "required": ["session_token"],
                    "properties": {
                        "session_token": { "type": "string", "format": "uuid" }
                    }
                },
                "ChangePasswordRequest": {
                    "type": "object",
                    "required": ["session_token", "current_password", "new_password"],
                    "properties": {
                        "session_token": { "type": "string", "format": "uuid" },
                        "current_password": { "type": "string", "format": "password" },
                        "new_password": { "type": "string", "format": "password" }
                    }
                },
                "CommandRequest": {
                    "type": "object",
                    "required": ["command", "args"],
                    "properties": {
                        "command": { "type": "string" },
                        "args": {
                            "type": "object",
                            "description": "Argumentos do comando. Comandos protegidos exigem session_token e podem receber active_company_id."
                        }
                    }
                },
                "CommandResponse": {
                    "type": "object",
                    "required": ["ok"],
                    "properties": {
                        "ok": { "type": "boolean" },
                        "result": {},
                        "error": { "type": "string", "nullable": true }
                    }
                }
            }
        },
        "paths": {
            "/health": {
                "get": {
                    "summary": "Health check público/local",
                    "responses": { "200": { "description": "OK" } }
                }
            },
            "/api/auth/login": {
                "post": {
                    "security": protected_security.clone(),
                    "summary": "Autenticar usuário para API ou mobile remoto",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/AuthLoginRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Resposta de autenticação" },
                        "401": { "description": "Credenciais, canal ou acesso rejeitado" }
                    }
                }
            },
            "/api/auth/restore": {
                "post": {
                    "security": protected_security.clone(),
                    "summary": "Restaurar sessão API/mobile",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SessionRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Sessão restaurada" },
                        "401": { "description": "Sessão inválida ou expirada" }
                    }
                }
            },
            "/api/auth/logout": {
                "post": {
                    "security": protected_security.clone(),
                    "summary": "Encerrar sessão API/mobile",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/SessionRequest" }
                            }
                        }
                    },
                    "responses": { "200": { "description": "Sessão encerrada" } }
                }
            },
            "/api/auth/change-password": {
                "post": {
                    "security": protected_security.clone(),
                    "summary": "Trocar senha usando a política configurada",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/ChangePasswordRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": { "description": "Senha alterada" },
                        "401": { "description": "Sessão ou senha atual inválida" }
                    }
                }
            },
            "/api/auth/password-policy": {
                "get": {
                    "security": protected_security.clone(),
                    "summary": "Consultar a política de senha efetiva",
                    "responses": {
                        "200": { "description": "Política de senha configurada" },
                        "401": { "description": "Token global inválido" }
                    }
                }
            },
            "/api/command": {
                "post": {
                    "security": protected_security.clone(),
                    "summary": "Executar comando autorizado pelo perfil e empresa do usuário",
                    "requestBody": {
                        "required": true,
                        "content": {
                            "application/json": {
                                "schema": { "$ref": "#/components/schemas/CommandRequest" }
                            }
                        }
                    },
                    "responses": {
                        "200": {
                            "description": "Comando processado",
                            "content": {
                                "application/json": {
                                    "schema": { "$ref": "#/components/schemas/CommandResponse" }
                                }
                            }
                        },
                        "400": { "description": "Comando, sessão, permissão ou empresa rejeitada" },
                        "401": { "description": "Token global inválido" }
                    }
                }
            },
            "/version": {
                "get": {
                    "security": protected_security.clone(),
                    "summary": "Versão da aplicação",
                    "responses": {
                        "200": { "description": "OK" },
                        "401": { "description": "Token inválido" }
                    }
                }
            },
            "/status": {
                "get": {
                    "security": protected_security.clone(),
                    "summary": "Status operacional",
                    "responses": { "200": { "description": "OK" } }
                }
            },
            "/app/meta": {
                "get": {
                    "security": protected_security.clone(),
                    "summary": "Metadados da aplicação",
                    "responses": { "200": { "description": "OK" } }
                }
            },
            "/features": {
                "get": {
                    "security": protected_security.clone(),
                    "summary": "Feature flags",
                    "responses": { "200": { "description": "OK" } }
                }
            },
            "/logs": {
                "get": {
                    "security": protected_security.clone(),
                    "summary": "Últimos logs",
                    "responses": { "200": { "description": "OK" } }
                }
            },
            "/openapi.json": {
                "get": {
                    "summary": "OpenAPI",
                    "responses": { "200": { "description": "JSON" } }
                }
            },
            docs_path: {
                "get": {
                    "summary": "Documentação Scalar em tema claro fixo",
                    "responses": { "200": { "description": "HTML" } }
                }
            }
        }
    })
}
