<script setup lang="ts">
import { reactive, ref } from "vue";
import { useRouter } from "vue-router";
import { appBranding } from "../config/appBranding";
import { resetCommandProvider } from "../core/invoker/CommandProviderFactory";
import {
  loadMobileConnection,
  saveMobileConnection,
  testMobileConnection,
  type MobileConnectionConfig,
} from "../core/mobile/MobileConnection";
import { useSessionStore } from "../stores/session";

const router = useRouter();
const session = useSessionStore();
const existing = loadMobileConnection();
const form = reactive<MobileConnectionConfig>({ ...existing });
const gatewayToken = ref("");
const testing = ref(false);
const message = ref("");
const error = ref("");

async function startLocal() {
  error.value = "";
  message.value = "";
  await session.logout({ silent: true });
  form.mode = "local";
  saveMobileConnection(form, "");
  resetCommandProvider();
  if (form.localAuthenticationRequired) {
    await router.replace("/login");
    return;
  }
  try {
    await session.loginAsLocalGuest();
    await router.replace("/");
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : "Falha ao iniciar o modo local.";
  }
}

async function validateRemote() {
  error.value = "";
  testing.value = true;
  try {
    const result = await testMobileConnection({ ...form, mode: "remote" }, gatewayToken.value);
    message.value = result.message;
    if (!result.ok) error.value = result.message;
    return result.ok;
  } finally {
    testing.value = false;
  }
}

async function startRemote() {
  error.value = "";
  message.value = "";
  if (!form.baseUrl.trim()) {
    error.value = "Informe a URL do desktop, servidor ou CloudPanel.";
    return;
  }
  const ok = await validateRemote();
  if (!ok) return;
  await session.logout({ silent: true });
  form.mode = "remote";
  saveMobileConnection(form, gatewayToken.value);
  resetCommandProvider();
  await router.replace("/login");
}
</script>

<template>
  <main class="mobile-start-page">
    <section class="mobile-start-card">
      <div class="badge">{{ appBranding.appName }} mobile</div>
      <h1>Como este aplicativo será usado?</h1>
      <p class="muted">
        A escolha pode ser alterada depois. O modo local mantém os dados no dispositivo;
        o modo remoto aplica o usuário, perfil e empresas autorizadas no servidor.
      </p>

      <div class="mode-grid">
        <article class="mode-card">
          <h2>Aplicativo local</h2>
          <p>SQLite privado no smartphone, sem depender do desktop ou da internet.</p>
          <label class="check-row">
            <input v-model="form.localAuthenticationRequired" type="checkbox" />
            Exigir login também no modo local
          </label>
          <button class="primary" type="button" :disabled="session.loading" @click="startLocal">
            {{ session.loading ? "Iniciando..." : "Usar no dispositivo" }}
          </button>
        </article>

        <article class="mode-card">
          <h2>Cliente do servidor</h2>
          <p>Conecta na API publicada pelo desktop, headless ou CloudPanel.</p>
          <div class="field">
            <label>Nome da conexão</label>
            <input v-model="form.connectionName" type="text" placeholder="Servidor principal" />
          </div>
          <div class="field">
            <label>URL da API</label>
            <input v-model="form.baseUrl" type="url" placeholder="https://app.exemplo.com" autocapitalize="none" />
          </div>
          <div class="field">
            <label>Header do token global</label>
            <input v-model="form.tokenHeader" type="text" placeholder="X-App-Token" autocapitalize="none" />
          </div>
          <div class="field">
            <label>Token global do gateway (opcional)</label>
            <input v-model="gatewayToken" type="password" autocomplete="off" />
            <small>O token fica apenas na sessão atual do aplicativo.</small>
          </div>
          <div class="actions">
            <button class="secondary" type="button" :disabled="testing" @click="validateRemote">
              {{ testing ? "Testando..." : "Testar conexão" }}
            </button>
            <button class="primary" type="button" :disabled="testing" @click="startRemote">
              Conectar e entrar
            </button>
          </div>
        </article>
      </div>

      <div v-if="message && !error" class="alert info">{{ message }}</div>
      <div v-if="error" class="alert error">{{ error }}</div>
    </section>
  </main>
</template>

<style scoped>
.mobile-start-page { min-height: 100vh; padding: 24px; display: grid; place-items: center; background: var(--surface-soft, #f4f7fb); }
.mobile-start-card { width: min(920px, 100%); background: var(--surface, #fff); border-radius: 22px; padding: clamp(20px, 4vw, 40px); box-shadow: 0 22px 70px rgba(15, 23, 42, .12); }
.mode-grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 18px; margin: 24px 0; }
.mode-card { border: 1px solid var(--border, #dbe2ea); border-radius: 16px; padding: 20px; display: grid; gap: 14px; align-content: start; }
.mode-card h2 { margin: 0; }
.mode-card p { margin: 0; color: var(--muted, #64748b); }
.check-row { display: flex; gap: 10px; align-items: center; }
.actions { display: flex; gap: 10px; flex-wrap: wrap; }
.field small { display: block; color: var(--muted, #64748b); margin-top: 4px; }
@media (max-width: 760px) { .mode-grid { grid-template-columns: 1fr; } .mobile-start-page { padding: 12px; } }
</style>
