#!/usr/bin/env python3
"""Valida estrutura, configuração, licenciamento e gates de um projeto Tauri/Rust."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import sys
import tomllib
from pathlib import Path


REQUIRED_FILES = (
    ".gitignore",
    "package.json",
    "package-lock.json",
    "src/config/projectConfig.ts",
    "src-tauri/Cargo.toml",
    "src-tauri/Cargo.lock",
    "src-tauri/tauri.conf.json",
    "src-tauri/src/bootstrap.rs",
    "src-tauri/src/legacy_data.rs",
    "src-tauri/src/storage_contract.rs",
    "src/pages/FirstAccessPage.vue",
    "src/pages/MobileStartPage.vue",
    "src/core/mobile/MobileConnection.ts",
    "src/core/invoker/MobileRemoteCommandProvider.ts",
    "docs/AUTHENTICATION_AND_MOBILE_ACCESS.md",
    "public/manifest.webmanifest",
    ".github/workflows/ci.yml",
    ".github/workflows/release.yml",
    ".github/workflows/cloudpanel-linux-release.yml",
    "scripts/release/collect-release-assets.mjs",
    "scripts/release/upload-release-assets.mjs",
    "scripts/release/write-build-status.mjs",
    "scripts/release/enforce-release-policy.mjs",
    "scripts/release/tests/release-pipeline.test.mjs",
    "scripts/linux/build-cloudpanel-release.sh",
    "tests/legacy-migration-harness/Cargo.toml",
    "scripts/windows/build-windows.ps1",
    "deploy/cloudpanel/.env.public.example",
    "deploy/cloudpanel/ports.sh",
)

REQUIRED_NPM_SCRIPTS = (
    "ci:version",
    "typecheck",
    "build:web",
    "fmt:rust:check",
    "lint:rust",
    "test:rust",
    "validate:web",
    "validate:rust",
    "validate:all",
    "release:ci",
    "release:dry",
    "release:collect",
    "release:upload",
    "build:windows:short-target",
    "build:windows:project-target",
    "build:windows:ci-fast",
)


MOBILE_MARKER_FILES = (
    "src-tauri/tauri.android.conf.json",
    "src-tauri/tauri.ios.conf.json",
    "src-tauri/mobile/mobile.config.json",
    "scripts/mobile/init-mobile.mjs",
    "scripts/mobile/generate-mobile-icons.mjs",
    "scripts/mobile/collect-android-artifact.mjs",
    "scripts/mobile/build-ios-unsigned.sh",
    "scripts/release/package-ios-unsigned.sh",
    ".github/workflows/mobile-android.yml",
    ".github/workflows/mobile-ios.yml",
)

MOBILE_REQUIRED_FILES = (
    *MOBILE_MARKER_FILES,
    "scripts/ci/run-tauri.mjs",
)

MOBILE_REQUIRED_NPM_SCRIPTS = (
    "tauri",
    "tauri:android:init",
    "tauri:android:dev",
    "tauri:android:build",
    "tauri:android:apk",
    "tauri:android:aab",
    "tauri:ios:init",
    "tauri:ios:dev",
    "tauri:ios:build",
    "tauri:ios:unsigned",
)

MOBILE_ANDROID_TARGETS = {
    "aarch64-linux-android",
    "armv7-linux-androideabi",
    "x86_64-linux-android",
    "i686-linux-android",
}

MOBILE_RELEASE_TARGETS = {
    "android-arm64-apk",
    "android-aab",
    "ios-arm64-unsigned-ipa",
}

MOBILE_FORBIDDEN_FEATURES = {
    "tauri/tray-icon",
    "tauri/image-png",
    "tauri/linux-libxdo",
    "tauri/macos-private-api",
}

SIGNING_SECRET_PATTERN = re.compile(
    r"secrets\.(?:APPLE|IOS|APP_STORE|ANDROID|GOOGLE_PLAY|KEYSTORE|P12|PROVISIONING)",
    re.IGNORECASE,
)

REQUIRED_CI_MARKERS = (
    "pull_request:",
    "workflow_dispatch:",
    "actionlint",
    "npm ci",
    "npm run ci:version",
    "npm run typecheck",
    "npm run build:web",
    "cargo fmt",
    "cargo check",
    "cargo clippy",
    "cargo test",
    "--locked",
    "Validate PowerShell syntax",
    "deploy/cloudpanel/ports.sh",
    "cloudpanel-headless:",
    "arch: x64",
    "arch: x86",
    "Build and package CloudPanel headless",
    "legacy-migration-harness/Cargo.toml",
)

REQUIRED_RELEASE_MARKERS = (
    "workflow_run:",
    "workflow_dispatch:",
    "contents: write",
    "cancel-in-progress: false",
    "tauri-apps/tauri-action@v1",
    "actions/upload-artifact@v7",
    "actions/download-artifact@v8",
    "x86_64-pc-windows-msvc",
    "i686-pc-windows-msvc",
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
    "collect-release-assets.mjs",
    "upload-release-assets.mjs",
    "CARGO_TARGET_DIR",
    "runner.temp }}/tauri-target",
    "SHA256SUMS.txt",
    "allow_partial_release",
    "Keep incomplete release as resumable draft",
    "--draft=false",
)

FORBIDDEN_HEADLESS_CRATES = {
    "tauri",
    "tauri-build",
    "tauri-runtime",
    "tauri-runtime-wry",
    "wry",
    "gtk",
    "gtk-sys",
    "gdk",
    "gdk-sys",
    "webkit2gtk",
    "webkit2gtk-sys",
    "tray-icon",
    "muda",
    "libappindicator",
    "libappindicator-sys",
}

TEXT_SUFFIXES = {
    ".css",
    ".html",
    ".js",
    ".json",
    ".jsx",
    ".md",
    ".mjs",
    ".rs",
    ".toml",
    ".ts",
    ".tsx",
    ".txt",
    ".vue",
    ".yaml",
    ".yml",
}

SKIP_DIRECTORIES = {
    ".git",
    "dist",
    "node_modules",
    "release",
    "target",
}

CONFLICT_PATTERN = re.compile(
    r"^\s*(?:<<<<<<<\s+\S.*|=======|>>>>>>>\s+\S.*)\s*$"
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Valida projeto Tauri 2 + Rust + Vue/TypeScript e seus workflows."
    )
    parser.add_argument("--project", required=True, type=Path)
    parser.add_argument(
        "--mode",
        choices=("static", "local", "ci"),
        default="static",
        help="static não executa builds; local/ci executam a matriz completa.",
    )
    parser.add_argument(
        "--allow-licensing",
        action="store_true",
        help="Aceita licenciamento ativo somente quando houve solicitação explícita.",
    )
    parser.add_argument(
        "--skip-npm-ci",
        action="store_true",
        help="Não reinstala node_modules nos modos local/ci.",
    )
    return parser.parse_args()


def read_json(path: Path, findings: list[str]) -> dict | None:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeDecodeError, json.JSONDecodeError) as exc:
        findings.append(f"JSON inválido em {path}: {exc}")
        return None
    if not isinstance(value, dict):
        findings.append(f"JSON raiz deve ser objeto em {path}")
        return None
    return value


def read_toml(path: Path, findings: list[str]) -> dict | None:
    try:
        with path.open("rb") as stream:
            value = tomllib.load(stream)
    except (OSError, tomllib.TOMLDecodeError) as exc:
        findings.append(f"TOML inválido em {path}: {exc}")
        return None
    return value


def iter_text_files(project: Path):
    for path in project.rglob("*"):
        if not path.is_file():
            continue
        relative = path.relative_to(project)
        if any(part in SKIP_DIRECTORIES for part in relative.parts):
            continue
        if path.suffix.lower() in TEXT_SUFFIXES or path.name in {
            ".env",
            ".env.example",
            "VERSION",
        }:
            yield path


def scan_conflicts(project: Path, findings: list[str]) -> None:
    for path in iter_text_files(project):
        try:
            content = path.read_text(encoding="utf-8")
        except (OSError, UnicodeDecodeError):
            continue
        for line_number, line in enumerate(content.splitlines(), 1):
            if CONFLICT_PATTERN.fullmatch(line):
                findings.append(
                    f"marcador de conflito em {path.relative_to(project)}:{line_number}"
                )


def extract_boolean(content: str, key: str) -> bool | None:
    match = re.search(rf"(?m)^\s*{re.escape(key)}\s*:\s*(true|false)\s*,?", content)
    if not match:
        return None
    return match.group(1) == "true"


def validate_licensing(
    project: Path, findings: list[str], *, allow_licensing: bool
) -> None:
    checks = (
        ("src/config/projectConfig.ts", "licensing", False),
        ("src/config/projectConfig.ts", "licensingStatus", False),
        ("src-tauri/src/core/mod.rs", "licensing", False),
    )
    values: list[tuple[str, bool]] = []
    for relative, key, _expected in checks:
        path = project / relative
        if not path.is_file():
            continue
        value = extract_boolean(path.read_text(encoding="utf-8"), key)
        if value is None:
            findings.append(f"flag {key!r} não encontrada em {relative}")
        else:
            values.append((f"{relative}:{key}", value))

    routes_path = project / "src-tauri" / "src" / "internal_api" / "routes.rs"
    if routes_path.is_file():
        routes = routes_path.read_text(encoding="utf-8")
        match = re.search(r'"licensing"\s*:\s*(true|false)', routes)
        if match:
            values.append(
                ("src-tauri/src/internal_api/routes.rs:licensing", match.group(1) == "true")
            )
        else:
            findings.append("metadado licensing não encontrado na API interna")

    commands_path = project / "src-tauri" / "src" / "commands" / "licensing.rs"
    if commands_path.is_file():
        commands = commands_path.read_text(encoding="utf-8")
        feature_match = re.search(
            r"const LICENSING_FEATURE_ENABLED:\s*bool\s*=\s*(true|false)\s*;",
            commands,
        )
        if feature_match:
            values.append(
                (
                    "src-tauri/src/commands/licensing.rs:LICENSING_FEATURE_ENABLED",
                    feature_match.group(1) == "true",
                )
            )
        else:
            findings.append("gate LICENSING_FEATURE_ENABLED ausente no backend Rust")

        disabled_match = re.search(
            r'"licensing_disabled"\.to_string\(\),\s*Value::from\((true|false)\)',
            commands,
        )
        if not disabled_match:
            findings.append("default licensing_disabled ausente no backend Rust")
        elif not allow_licensing and disabled_match.group(1) != "true":
            findings.append(
                "licenciamento deve iniciar desabilitado: "
                "src-tauri/src/commands/licensing.rs:licensing_disabled"
            )

    if not allow_licensing:
        for label, enabled in values:
            if enabled:
                findings.append(f"licenciamento deve permanecer desligado: {label}")


def validate_versions(
    project: Path,
    package: dict | None,
    cargo: dict | None,
    tauri: dict | None,
    findings: list[str],
) -> None:
    version_path = project / "VERSION"
    version_file = (
        version_path.read_text(encoding="utf-8").strip() if version_path.is_file() else None
    )
    versions = {
        "package.json": package.get("version") if package else None,
        "src-tauri/Cargo.toml": cargo.get("package", {}).get("version") if cargo else None,
        "src-tauri/tauri.conf.json": tauri.get("version") if tauri else None,
        "VERSION": version_file,
    }
    present = {str(value) for value in versions.values() if value is not None}
    if len(present) > 1:
        details = ", ".join(f"{name}={value!r}" for name, value in versions.items())
        findings.append(f"versões divergentes: {details}")
    if not present:
        findings.append("versão do projeto não pôde ser determinada")


def validate_package(package: dict | None, findings: list[str]) -> None:
    if package is None:
        return
    scripts = package.get("scripts")
    if not isinstance(scripts, dict):
        findings.append("package.json não possui objeto scripts")
        return
    for script in REQUIRED_NPM_SCRIPTS:
        if not isinstance(scripts.get(script), str) or not scripts[script].strip():
            findings.append(f"script npm obrigatório ausente: {script}")


def validate_workflows(project: Path, findings: list[str]) -> None:
    workflows_dir = project / ".github" / "workflows"
    if not workflows_dir.is_dir():
        findings.append("diretório .github/workflows ausente")
        return

    workflows = sorted(
        path
        for path in workflows_dir.iterdir()
        if path.is_file() and path.suffix.lower() in {".yml", ".yaml"}
    )
    if not workflows:
        findings.append("nenhum workflow GitHub Actions encontrado")
        return

    for path in workflows:
        content = path.read_text(encoding="utf-8")
        relative = path.relative_to(project)
        if "\t" in content:
            findings.append(f"tabulação inválida em workflow: {relative}")
        for required in ("name:", "on:", "jobs:"):
            if not re.search(rf"(?m)^{re.escape(required)}", content):
                findings.append(f"{relative} não contém chave raiz {required}")

    ci_workflow = project / ".github" / "workflows" / "ci.yml"
    if ci_workflow.is_file():
        content = ci_workflow.read_text(encoding="utf-8")
        for marker in REQUIRED_CI_MARKERS:
            if marker not in content:
                findings.append(f"workflow de CI não contém: {marker}")

    release_workflow = project / ".github" / "workflows" / "release.yml"
    if not release_workflow.is_file():
        return
    content = release_workflow.read_text(encoding="utf-8")
    for marker in REQUIRED_RELEASE_MARKERS:
        if marker not in content:
            findings.append(f"workflow de release não contém: {marker}")


def validate_required_files(project: Path, findings: list[str]) -> None:
    for relative in REQUIRED_FILES:
        path = project / relative
        if not path.is_file():
            findings.append(f"arquivo obrigatório ausente: {relative}")
        elif path.stat().st_size == 0:
            findings.append(f"arquivo obrigatório vazio: {relative}")


def validate_gitignore_contract(project: Path, findings: list[str]) -> None:
    gitignore_path = project / ".gitignore"
    if not gitignore_path.is_file():
        return

    lines = [
        line.strip()
        for line in gitignore_path.read_text(encoding="utf-8").splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    ]
    if "release/" in lines:
        findings.append(
            ".gitignore não deve usar release/ sem âncora; esse padrão ignora scripts/release"
        )
    if "/release/" not in lines:
        findings.append(
            ".gitignore deve ignorar somente o diretório de saída raiz com /release/"
        )


def validate_headless_contract(
    project: Path, cargo: dict | None, findings: list[str]
) -> None:
    if cargo is None:
        return
    dependencies = cargo.get("dependencies", {})
    build_dependencies = cargo.get("build-dependencies", {})
    features = cargo.get("features", {})
    tauri = dependencies.get("tauri")
    tauri_build = build_dependencies.get("tauri-build")
    desktop = features.get("desktop", [])
    if not isinstance(tauri, dict) or tauri.get("optional") is not True:
        findings.append("tauri deve ser dependência opcional")
    elif tauri.get("default-features") is not False:
        findings.append("tauri deve usar default-features = false")
    if not isinstance(tauri_build, dict) or tauri_build.get("optional") is not True:
        findings.append("tauri-build deve ser build-dependency opcional")
    if not isinstance(desktop, list) or not {"dep:tauri", "dep:tauri-build"}.issubset(desktop):
        findings.append("feature desktop deve ativar dep:tauri e dep:tauri-build")

    build_gate = (
        '#[cfg(any(feature = "desktop", feature = "mobile"))]'
        if mobile_support_enabled(project)
        else '#[cfg(feature = "desktop")]'
    )
    textual_checks = {
        "src-tauri/build.rs": (build_gate, "tauri_build::build()"),
        "src-tauri/src/main.rs": ('#[cfg(not(feature = "desktop"))]',),
        "scripts/linux/build-cloudpanel-release.sh": (
            "--edges normal,build",
            "--no-default-features",
            "tauri-build",
            "webkit2gtk",
            "libappindicator",
        ),
    }
    for relative, markers in textual_checks.items():
        path = project / relative
        if not path.is_file():
            continue
        content = path.read_text(encoding="utf-8")
        for marker in markers:
            if marker not in content:
                findings.append(f"contrato headless ausente em {relative}: {marker}")


def mobile_support_enabled(project: Path) -> bool:
    return any((project / relative).exists() for relative in MOBILE_MARKER_FILES)


def require_markers(
    content: str, relative: str, markers: tuple[str, ...], findings: list[str]
) -> None:
    for marker in markers:
        if marker not in content:
            findings.append(f"contrato mobile ausente em {relative}: {marker}")


def validate_mobile_contract(
    project: Path,
    package: dict | None,
    cargo: dict | None,
    tauri: dict | None,
    findings: list[str],
) -> None:
    """Valida mobile somente quando o projeto já possui algum marcador de migração."""
    if not mobile_support_enabled(project):
        return

    for relative in MOBILE_REQUIRED_FILES:
        path = project / relative
        if not path.is_file():
            findings.append(f"arquivo mobile obrigatório ausente: {relative}")
        elif path.stat().st_size == 0:
            findings.append(f"arquivo mobile obrigatório vazio: {relative}")

    scripts = package.get("scripts", {}) if package else {}
    if not isinstance(scripts, dict):
        scripts = {}
    for script in MOBILE_REQUIRED_NPM_SCRIPTS:
        value = scripts.get(script)
        if not isinstance(value, str) or not value.strip():
            findings.append(f"script npm mobile obrigatório ausente: {script}")
    if isinstance(scripts.get("tauri"), str) and "scripts/ci/run-tauri.mjs" not in scripts["tauri"]:
        findings.append("script npm tauri deve preservar scripts/ci/run-tauri.mjs")

    features = cargo.get("features", {}) if cargo else {}
    default_features = features.get("default", []) if isinstance(features, dict) else []
    desktop = features.get("desktop", []) if isinstance(features, dict) else []
    mobile = features.get("mobile", []) if isinstance(features, dict) else []
    if default_features != []:
        findings.append(
            "features Cargo default devem permanecer vazias; desktop/mobile são selecionadas por runtime"
        )
    base_features = tauri.get("build", {}).get("features", []) if tauri else []
    if base_features != ["desktop"]:
        findings.append("tauri.conf.json deve ativar somente a feature desktop")
    if not isinstance(desktop, list) or not {"dep:tauri", "dep:tauri-build"}.issubset(desktop):
        findings.append("feature desktop deve ser preservada com Tauri e tauri-build")
    mobile_required = {"dep:tauri", "dep:tauri-build", "tauri/wry", "tauri/custom-protocol"}
    if not isinstance(mobile, list) or not mobile_required.issubset(mobile):
        findings.append(
            "feature mobile deve ativar tauri, tauri-build, Wry mobile e custom-protocol"
        )
    elif MOBILE_FORBIDDEN_FEATURES.intersection(mobile):
        found = ", ".join(sorted(MOBILE_FORBIDDEN_FEATURES.intersection(mobile)))
        findings.append(f"feature mobile contém recursos exclusivamente desktop: {found}")

    textual_contracts: dict[str, tuple[str, ...]] = {
        "src-tauri/build.rs": (
            '#[cfg(any(feature = "desktop", feature = "mobile"))]',
            "tauri_build::build()",
        ),
        "src-tauri/src/lib.rs": (
            '#[cfg(all(any(target_os = "android", target_os = "ios"), feature = "desktop"))]',
            "compile_error!",
            '#[cfg(all(feature = "mobile", not(feature = "desktop")))]',
            "#[cfg_attr(mobile, tauri::mobile_entry_point)]",
            "app_local_data_dir()",
        ),
        "src/router/index.ts": (
            "isNativeMobileRuntime",
            "desktopOnly",
            "Rota exclusiva do runtime desktop bloqueada no mobile nativo",
        ),
        "src/config/navigation.ts": (
            "isNativeMobileRuntime",
            "desktopOnly",
            "visibleMenuItems",
        ),
        "scripts/mobile/init-mobile.mjs": (
            "src-tauri",
            "gen",
            "android",
            "apple",
            "run-tauri.mjs",
            "generate-mobile-icons.mjs",
            "--ci",
            "--skip-targets-install",
        ),
        "scripts/mobile/generate-mobile-icons.mjs": (
            "1024",
            "sha256",
            "run-tauri.mjs",
            "'icon'",
            "preserveDesktopIcons",
            "restoreDesktopIcons",
            ".mobile-branding.json",
        ),
        "scripts/mobile/collect-android-artifact.mjs": (
            "arm64-v8a",
            ".apk",
            ".aab",
            "sha256",
            "isJarSignatureEntry",
            "META-INF",
            "AAB contém assinatura JAR inesperada",
        ),
        "scripts/mobile/build-ios-unsigned.sh": (
            "Darwin",
            "aarch64",
            "CODE_SIGNING_ALLOWED=NO",
            "--features mobile",
            "--no-sign",
            "--archive-only",
            "O build Tauri iOS não assinado falhou",
            "prepare-tauri-resources.mjs",
            "app-path.txt",
            ".build-started-at",
            "-newer",
            'rm -rf "${GEN_DIR}/build"',
        ),
        "scripts/release/package-ios-unsigned.sh": (
            "Payload",
            "Info.plist",
            "CFBundleExecutable",
            "Mach-O",
            "arm64",
            "platform IOSSIMULATOR",
            "platform IOS",
            "mktemp -d",
            "zip -qry",
            "shasum -a 256",
        ),
    }
    for relative, markers in textual_contracts.items():
        path = project / relative
        if path.is_file():
            require_markers(path.read_text(encoding="utf-8"), relative, markers, findings)

    ios_builder_path = project / "scripts/mobile/build-ios-unsigned.sh"
    if ios_builder_path.is_file():
        ios_builder = ios_builder_path.read_text(encoding="utf-8")
        if re.search(r"(?m)^\s*xcodebuild(?:\s|\\)", ios_builder):
            findings.append(
                "build iOS não assinado não deve executar xcodebuild diretamente; "
                "o archive deve permanecer sob o processo do Tauri CLI"
            )
        if "is_signing_failure" in ios_builder:
            findings.append(
                "fallback legado de assinatura iOS detectado; use --no-sign --archive-only"
            )

    android_config = read_json(project / "src-tauri/tauri.android.conf.json", findings) if (
        project / "src-tauri/tauri.android.conf.json"
    ).is_file() else None
    ios_config = read_json(project / "src-tauri/tauri.ios.conf.json", findings) if (
        project / "src-tauri/tauri.ios.conf.json"
    ).is_file() else None
    mobile_config = read_json(project / "src-tauri/mobile/mobile.config.json", findings) if (
        project / "src-tauri/mobile/mobile.config.json"
    ).is_file() else None

    for label, config in (("Android", android_config), ("iOS", ios_config)):
        platform_features = config.get("build", {}).get("features", []) if config else []
        if platform_features != ["mobile"]:
            findings.append(f"configuração {label} deve ativar somente a feature mobile")

    if mobile_config:
        frontend = mobile_config.get("frontend", {})
        if frontend.get("strategy") != "local-bundled":
            findings.append("mobile deve usar o frontend local-bundled")
        if frontend.get("remoteWebView") is not False:
            findings.append("mobile não pode ser configurado como WebView remoto")
        if frontend.get("localDatabase") != "sqlite":
            findings.append("mobile deve manter SQLite local")
        if frontend.get("dashboardsAndCharts") != "local":
            findings.append("dashboards e gráficos mobile devem executar localmente")
        if frontend.get("standaloneReady") is not True or frontend.get("restClientReady") is not True:
            findings.append("mobile deve preservar os modos independente e cliente REST")

        authentication = mobile_config.get("authentication", {})
        if authentication.get("desktopBootstrap") != "internal":
            findings.append("bootstrap administrativo deve permanecer interno ao backend")
        if authentication.get("localLoginRequiredByDefault") is not False:
            findings.append("login mobile local deve ser opcional por padrão")
        if authentication.get("remoteLoginRequired") is not True:
            findings.append("modo mobile remoto deve exigir autenticação por usuário")
        operation_modes = mobile_config.get("operationModes", {})
        if operation_modes.get("local", {}).get("authentication") != "optional":
            findings.append("modo mobile local deve declarar autenticação opcional")
        remote_authorization = set(
            operation_modes.get("remote", {}).get("authorization", [])
        )
        if remote_authorization != {"profile", "permissions", "companies"}:
            findings.append(
                "modo mobile remoto deve aplicar perfil, permissões e empresas"
            )

        android = mobile_config.get("android", {})
        ios = mobile_config.get("ios", {})
        if android.get("generatedProject") != "src-tauri/gen/android":
            findings.append("diretório Android deve ser src-tauri/gen/android")
        if set(android.get("rustTargets", [])) != MOBILE_ANDROID_TARGETS:
            findings.append("lista de targets Rust Android está incompleta ou divergente")
        if android.get("primaryRustTarget") != "aarch64-linux-android":
            findings.append("target Android principal deve ser aarch64-linux-android")
        if set(android.get("artifacts", [])) != {"apk", "aab"}:
            findings.append("Android deve declarar APK e AAB")
        if ios.get("generatedProject") != "src-tauri/gen/apple":
            findings.append("diretório iOS deve ser src-tauri/gen/apple")
        if ios.get("primaryRustTarget") != "aarch64-apple-ios":
            findings.append("target iOS principal deve ser aarch64-apple-ios")
        if ios.get("unsignedIpa") is not True:
            findings.append("iOS deve declarar empacotamento de IPA não assinado")

        android_identifier = android.get("package")
        ios_identifier = ios.get("bundleIdentifier")
        if android_config and android_config.get("identifier") != android_identifier:
            findings.append("package Android diverge de tauri.android.conf.json")
        if ios_config and ios_config.get("identifier") != ios_identifier:
            findings.append("bundle identifier iOS diverge de tauri.ios.conf.json")
        if tauri and mobile_config.get("identifier") != tauri.get("identifier"):
            findings.append("identifier base mobile diverge de tauri.conf.json")

        assets = mobile_config.get("assets", {})
        mobile_icon_paths: list[Path] = []
        for key in ("androidIconSource", "iosIconSource"):
            relative = assets.get(key)
            if not isinstance(relative, str) or not (project / relative).is_file():
                findings.append(f"asset mobile personalizado ausente: {key}")
                continue
            icon_path = project / relative
            mobile_icon_paths.append(icon_path)
            try:
                raw = icon_path.read_bytes()
                png_signature = bytes((137, 80, 78, 71, 13, 10, 26, 10))
                if len(raw) < 24 or raw[:8] != png_signature:
                    findings.append(f"asset mobile deve ser PNG válido: {relative}")
                else:
                    width = int.from_bytes(raw[16:20], "big")
                    height = int.from_bytes(raw[20:24], "big")
                    if (width, height) != (1024, 1024):
                        findings.append(
                            f"asset mobile deve medir 1024x1024: {relative} "
                            f"mede {width}x{height}"
                        )
            except OSError as exc:
                findings.append(f"não foi possível validar asset mobile {relative}: {exc}")
        if len(mobile_icon_paths) == 2:
            hashes = [hashlib.sha256(path.read_bytes()).hexdigest() for path in mobile_icon_paths]
            if hashes[0] != hashes[1]:
                findings.append(
                    "fontes de ícone Android e iOS devem preservar a mesma identidade visual"
                )

    android_workflow_path = project / ".github/workflows/mobile-android.yml"
    ios_workflow_path = project / ".github/workflows/mobile-ios.yml"
    release_workflow_path = project / ".github/workflows/release.yml"
    collector_path = project / "scripts/release/collect-release-assets.mjs"
    workflow_contracts = {
        android_workflow_path: (
            'NODE_VERSION: "24"',
            "actions/setup-java@v5",
            "android-actions/setup-android@v3",
            "ANDROID_NDK_VERSION",
            "GITHUB_PATH",
            "build-tools/${ANDROID_BUILD_TOOLS}",
            "npm ci",
            "aarch64-linux-android",
            "armv7-linux-androideabi",
            "x86_64-linux-android",
            "i686-linux-android",
            "--apk --debug --ci",
            "--aab --ci",
            "collect-android-artifact.mjs",
            "write-build-status.mjs",
            "--features mobile",
            "build:web:tauri",
            "actions/upload-artifact@v7",
        ),
        ios_workflow_path: (
            'NODE_VERSION: "24"',
            "runs-on: macos-15",
            "aarch64-apple-ios",
            "Validate Xcode and CocoaPods",
            "brew install cocoapods",
            "pod --version",
            "npm ci",
            "build-ios-unsigned.sh",
            "package-ios-unsigned.sh",
            "write-build-status.mjs",
            "build:web:tauri",
            "actions/upload-artifact@v7",
        ),
        release_workflow_path: (
            "android-arm64-apk",
            "android-aab",
            "ios-arm64-unsigned-ipa",
            "Validate Xcode and CocoaPods",
            "brew install cocoapods",
            "pod --version",
            "write-build-status.mjs",
            "release-android-arm64-apk",
            "release-android-aab",
            "release-ios-arm64-unsigned-ipa",
            "- android",
            "- ios",
        ),
        collector_path: (
            "'.apk'",
            "'.aab'",
            "'.ipa'",
            *tuple(sorted(MOBILE_RELEASE_TARGETS)),
        ),
    }
    for path, markers in workflow_contracts.items():
        if not path.is_file():
            continue
        content = path.read_text(encoding="utf-8")
        require_markers(content, str(path.relative_to(project)), markers, findings)
        if SIGNING_SECRET_PATTERN.search(content):
            findings.append(f"credencial de assinatura mobile encontrada em {path.relative_to(project)}")

    ci_path = project / ".github/workflows/ci.yml"
    if ci_path.is_file():
        ci = ci_path.read_text(encoding="utf-8")
        require_markers(
            ci,
            ".github/workflows/ci.yml",
            (
                "node --check scripts/mobile/generate-mobile-icons.mjs",
                "bash -n scripts/mobile/build-ios-unsigned.sh",
                "bash -n scripts/release/package-ios-unsigned.sh",
                "cargo tree --manifest-path src-tauri/Cargo.toml --locked --no-default-features",
                "cargo build --manifest-path src-tauri/Cargo.toml --locked --no-default-features",
            ),
            findings,
        )
        if "sdkmanager" in ci or "ANDROID_NDK_VERSION" in ci:
            findings.append("CI geral não deve instalar Android SDK/NDK; use somente o job mobile")


def validate_bootstrap_and_migration_contract(project: Path, findings: list[str]) -> None:
    if (project / ".bootstrap-admin.local").exists():
        findings.append("credencial bootstrap não pode existir no código-fonte ou pacote")

    forbidden_sources = (
        "src/config/projectConfig.ts",
        "src-tauri/src/migrations.rs",
        "src-tauri/src/core/mod.rs",
        "src-tauri/src/internal_api/app_commands.rs",
    )
    for relative in forbidden_sources:
        path = project / relative
        if path.is_file() and ("admin" + "123") in path.read_text(encoding="utf-8"):
            findings.append(f"credencial pública encontrada em {relative}")

    required_markers = {
        "src-tauri/src/bootstrap.rs": (
            "OsRng",
            "resolve_internal",
            "MANIFESTADOR_NFE_BOOTSTRAP_ADMIN_PASSWORD",
            "remove_legacy_file",
            "LEGACY_BOOTSTRAP_FILE_NAME",
        ),
        "src-tauri/src/legacy_data.rs": ("PRAGMA integrity_check", "backups", "rollback", "source_checksum_sha256", "CRITICAL_MIGRATION_TABLES"),
        "src-tauri/src/storage_contract.rs": ("CURRENT_LOCAL_DATA_DIR", "LEGACY_LOCAL_DATA_DIRS", "LEGACY_SQLITE_DATABASE_FILE_NAMES", "CRITICAL_MIGRATION_TABLES"),
        "src-tauri/src/security.rs": ("PasswordPolicy", "PASSWORD_MIN_ALLOWED", "load_password_policy", "save_password_policy"),
        "src-tauri/src/commands/auth.rs": (
            "load_session_identity",
            "auth_initialize_master_password",
            "auth_local_guest",
            "authorize_api_command",
            "não está autorizado para execução pela API remota",
            "api:access",
            "mobile:access",
            "senha_provisoria",
            "Credenciais inválidas.",
        ),
        "src-tauri/src/internal_api/app_commands.rs": (
            "public_or_first_access_command",
            "authorize_api_command",
            "company_allowed",
            "auth_login_endpoint",
            "Canal remoto inválido. Use api ou mobile",
            "user_record_for_api",
            "O usuário master da sessão não pode ser inativado nem perder o perfil master",
            "auth_password_policy_endpoint",
            'payload.remove("senha_hash")',
        ),
        "src-tauri/src/migrations.rs": (
            "LOWER(login) = 'admin' OR administrador = 1",
            '("usuarios", "administrador", "INTEGER NOT NULL DEFAULT 0")',
            "(\"usuarios\", \"created_at\", \"TEXT NOT NULL DEFAULT ''\")",
            "master_is_provisional",
            "environment_password_configured",
            "ensure_mobile_guest_seed",
            "api:access",
            "mobile:access",
        ),
        "src-tauri/src/internal_api/docs.rs": (
            '"/api/auth/login"',
            '"/api/auth/restore"',
            '"/api/auth/logout"',
            '"/api/auth/change-password"',
            '"/api/auth/password-policy"',
            '"/api/command"',
            "perfil",
            "empresa",
        ),
        "src/router/index.ts": ('path: "/primeiro-acesso"', 'path: "/mobile-inicio"', "senha_provisoria"),
        "src/pages/LoginPage.vue": ("Informe suas credenciais para acessar o sistema.", "Continuar sem login"),
        "src/core/invoker/MobileRemoteCommandProvider.ts": (
            "/api/auth/login",
            "/api/auth/password-policy",
            "/api/command",
            "session_token",
        ),
        "src/core/mobile/MobileConnection.ts": ("localAuthenticationRequired", "sessionStorage", "remote"),
    }
    for relative, markers in required_markers.items():
        path = project / relative
        if not path.is_file():
            continue
        content = path.read_text(encoding="utf-8")
        for marker in markers:
            if marker not in content:
                findings.append(f"contrato de bootstrap/migração ausente em {relative}: {marker}")

    bootstrap_path = project / "src-tauri" / "src" / "bootstrap.rs"
    bootstrap_override = "_".join(("TEMPLATE", "APP", "BOOTSTRAP", "FILE"))
    if bootstrap_path.is_file():
        bootstrap_source = bootstrap_path.read_text(encoding="utf-8")
        if bootstrap_override in bootstrap_source:
            findings.append("credencial bootstrap não pode aceitar caminho configurável")
        for forbidden in ("OpenOptions", "create_new(true)", "write_all(", "File::create"):
            if forbidden in bootstrap_source:
                findings.append(
                    f"bootstrap interno não pode persistir senha em arquivo: {forbidden}"
                )
        if "pub fn legacy_file_exists" in bootstrap_source:
            findings.append("bootstrap não deve expor leitura de credencial legada")

    auth_path = project / "src-tauri" / "src" / "commands" / "auth.rs"
    if auth_path.is_file():
        auth_source = auth_path.read_text(encoding="utf-8")
        guest_command = re.search(
            r'#\[cfg\(feature = "mobile"\)\]\s*#\[tauri::command\]\s*pub fn auth_local_guest',
            auth_source,
        )
        if not guest_command:
            findings.append(
                "auth_local_guest deve existir somente quando a feature mobile estiver ativa"
            )

    lib_path = project / "src-tauri" / "src" / "lib.rs"
    if lib_path.is_file():
        library = lib_path.read_text(encoding="utf-8")
        desktop_start = library.find('#[cfg(feature = "desktop")]\npub fn run()')
        mobile_start = library.find(
            '#[cfg(all(feature = "mobile", not(feature = "desktop")))]'
        )
        if desktop_start >= 0 and mobile_start > desktop_start:
            desktop_handler = library[desktop_start:mobile_start]
            if "commands::auth::auth_local_guest" in desktop_handler:
                findings.append(
                    "runtime desktop não pode expor o comando de acesso local sem login"
                )

    config_path = project / "src" / "config" / "projectConfig.ts"
    if config_path.is_file():
        config = config_path.read_text(encoding="utf-8")
        admin_block = re.search(r"defaultAdmin:\s*\{(?P<body>.*?)\n\s*\},", config, re.DOTALL)
        if admin_block and re.search(r"(?m)^\s*password\s*:", admin_block.group("body")):
            findings.append("defaultAdmin não pode conter senha fixa")


def run_command(project: Path, command: list[str]) -> bool:
    print(f"\n$ {' '.join(command)}")
    completed = subprocess.run(command, cwd=project, check=False)
    if completed.returncode != 0:
        print(f"FALHA: comando retornou {completed.returncode}", file=sys.stderr)
        return False
    return True


def run_cargo_tree_guard(project: Path, command: list[str]) -> bool:
    guarded = [*command, "--prefix", "none", "--format", "{p}"]
    print(f"\n$ {' '.join(guarded)}")
    completed = subprocess.run(
        guarded,
        cwd=project,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if completed.returncode != 0:
        print(completed.stdout, end="")
        print(completed.stderr, end="", file=sys.stderr)
        print(f"FALHA: cargo tree retornou {completed.returncode}", file=sys.stderr)
        return False
    packages = {
        line.split()[0]
        for line in completed.stdout.splitlines()
        if line.strip()
    }
    forbidden = sorted(packages & FORBIDDEN_HEADLESS_CRATES)
    if forbidden:
        print(
            "FALHA: grafo headless contém crates proibidas: " + ", ".join(forbidden),
            file=sys.stderr,
        )
        return False
    print("OK: grafo headless sem crates Tauri/GTK/WebKit proibidas.")
    return True


def run_full_matrix(project: Path, *, mode: str, skip_npm_ci: bool) -> int:
    required_tools = ("npm", "cargo")
    missing = [tool for tool in required_tools if shutil.which(tool) is None]
    if missing:
        print(f"FALHA: ferramentas ausentes: {', '.join(missing)}", file=sys.stderr)
        return 2

    commands: list[list[str]] = []
    if not skip_npm_ci:
        commands.append(["npm", "ci"])
    commands.extend(
        [
            ["npm", "run", "ci:version"],
            ["npm", "run", "typecheck"],
            ["npm", "run", "build:web"],
            ["node", "--test", "scripts/release/tests/release-pipeline.test.mjs"],
            [
                "cargo",
                "generate-lockfile",
                "--manifest-path",
                "src-tauri/Cargo.toml",
            ],
            [
                "cargo",
                "fmt",
                "--manifest-path",
                "src-tauri/Cargo.toml",
                "--all",
                "--check",
            ],
            [
                "cargo",
                "check",
                "--manifest-path",
                "src-tauri/Cargo.toml",
                "--locked",
                "--all-targets",
                "--all-features",
            ],
            [
                "cargo",
                "clippy",
                "--manifest-path",
                "src-tauri/Cargo.toml",
                "--locked",
                "--all-targets",
                "--all-features",
                "--",
                "-D",
                "warnings",
            ],
            [
                "cargo",
                "test",
                "--manifest-path",
                "src-tauri/Cargo.toml",
                "--locked",
                "--all-targets",
                "--all-features",
            ],
            [
                "cargo",
                "tree",
                "--manifest-path",
                "src-tauri/Cargo.toml",
                "--locked",
                "--no-default-features",
                "--edges",
                "normal,build",
            ],
            [
                "cargo",
                "build",
                "--manifest-path",
                "src-tauri/Cargo.toml",
                "--locked",
                "--no-default-features",
            ],
            [
                "cargo",
                "tree",
                "--manifest-path",
                "src-tauri/Cargo.toml",
                "--locked",
                "--no-default-features",
                "--features",
                "mysql-db,postgres-db",
                "--edges",
                "normal,build",
            ],
            [
                "cargo",
                "build",
                "--manifest-path",
                "src-tauri/Cargo.toml",
                "--locked",
                "--no-default-features",
                "--features",
                "mysql-db,postgres-db",
            ],
            [
                "cargo",
                "run",
                "--manifest-path",
                "tests/legacy-migration-harness/Cargo.toml",
                "--locked",
            ],
        ]
    )
    for command in commands:
        runner = run_cargo_tree_guard if command[:2] == ["cargo", "tree"] else run_command
        if not runner(project, command):
            return 1

    if mode == "ci":
        if not sys.platform.startswith("linux"):
            print("FALHA: modo ci exige host Linux para validar CloudPanel x64/x86.", file=sys.stderr)
            return 2
        if not run_command(
            project,
            [
                "env",
                "BUILD_WEB=false",
                "bash",
                "scripts/linux/build-cloudpanel-release.sh",
                "--all",
            ],
        ):
            return 1
        release_dir = project / "release" / "cloudpanel"
        archives = sorted(release_dir.glob("*-linux-x*.tar.gz"))
        checksums = sorted(release_dir.glob("*-linux-x*.tar.gz.sha256"))
        if len(archives) < 2 or len(checksums) < 2:
            print(
                "FALHA: modo ci não produziu TAR e SHA-256 para CloudPanel x64/x86.",
                file=sys.stderr,
            )
            return 1
    return 0


def main() -> int:
    args = parse_args()
    project = args.project.expanduser().resolve()
    if not project.is_dir():
        print(f"ERRO: projeto não encontrado: {project}", file=sys.stderr)
        return 2

    findings: list[str] = []
    validate_required_files(project, findings)
    validate_gitignore_contract(project, findings)
    scan_conflicts(project, findings)

    package = read_json(project / "package.json", findings) if (project / "package.json").is_file() else None
    read_json(project / "package-lock.json", findings) if (project / "package-lock.json").is_file() else None
    tauri = (
        read_json(project / "src-tauri" / "tauri.conf.json", findings)
        if (project / "src-tauri" / "tauri.conf.json").is_file()
        else None
    )
    read_json(project / "public" / "manifest.webmanifest", findings) if (
        project / "public" / "manifest.webmanifest"
    ).is_file() else None
    cargo = (
        read_toml(project / "src-tauri" / "Cargo.toml", findings)
        if (project / "src-tauri" / "Cargo.toml").is_file()
        else None
    )

    validate_package(package, findings)
    validate_versions(project, package, cargo, tauri, findings)
    validate_headless_contract(project, cargo, findings)
    validate_mobile_contract(project, package, cargo, tauri, findings)
    validate_bootstrap_and_migration_contract(project, findings)
    is_template_source = (project / ".tauri-template-source").is_file()
    if not is_template_source:
        validate_licensing(project, findings, allow_licensing=args.allow_licensing)
    validate_workflows(project, findings)

    if findings:
        print(f"FALHA: {len(findings)} problema(s) estrutural(is) encontrado(s).")
        for finding in findings:
            print(f"- {finding}")
        return 1

    print("OK: estrutura, configurações, licenciamento, mobile opcional e workflows validados.")
    if args.mode == "static":
        return 0
    return run_full_matrix(project, mode=args.mode, skip_npm_ci=args.skip_npm_ci)


if __name__ == "__main__":
    raise SystemExit(main())
