<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import { useRouter } from "vue-router";
import logoMark from "../assets/branding/logo-mark.png";
import { appBranding } from "../config/appBranding";
import { useSessionStore, type BootstrapStatus, type PasswordPolicy } from "../stores/session";

const router = useRouter();
const session = useSessionStore();
const form = reactive({ currentPassword: "", newPassword: "", confirmation: "" });
const error = ref("");
const loadingStatus = ref(true);
const bootstrapStatus = ref<BootstrapStatus | null>(null);
const policy = ref<PasswordPolicy>({
  min_length: 8,
  max_length: 128,
  min_allowed: 6,
  max_allowed: 128,
  require_letter: true,
  require_number: true,
  require_uppercase: false,
  require_lowercase: false,
  require_symbol: false,
});

const validConfirmation = computed(() => form.newPassword === form.confirmation);
const isLocalInitialization = computed(() => !session.isAuthenticated && Boolean(bootstrapStatus.value?.requires_setup));
const requirements = computed(() => {
  const items = [`${policy.value.min_length} a ${policy.value.max_length} caracteres`];
  if (policy.value.require_letter) items.push("uma letra");
  if (policy.value.require_number) items.push("um número");
  if (policy.value.require_uppercase) items.push("uma maiúscula");
  if (policy.value.require_lowercase) items.push("uma minúscula");
  if (policy.value.require_symbol) items.push("um símbolo");
  return items.join(", ");
});

onMounted(async () => {
  try {
    if (session.isAuthenticated) {
      policy.value = await session.passwordPolicy();
    } else {
      bootstrapStatus.value = await session.bootstrapStatus();
      policy.value = bootstrapStatus.value.password_policy || await session.passwordPolicy();
    }
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : "Falha ao carregar a política de senha.";
  } finally {
    loadingStatus.value = false;
  }
});

async function submit() {
  error.value = "";
  if (!validConfirmation.value) {
    error.value = "A confirmação não corresponde à nova senha.";
    return;
  }
  try {
    if (isLocalInitialization.value) {
      await session.initializeMasterPassword(form.newPassword);
    } else {
      await session.changePassword(form.currentPassword, form.newPassword);
    }
    await router.replace("/");
  } catch (caught) {
    error.value = caught instanceof Error ? caught.message : "Falha ao alterar senha.";
  }
}

async function logout() {
  await session.logout();
  await router.replace("/login");
}
</script>

<template>
  <main class="login-page">
    <section class="login-box first-access-card" aria-labelledby="first-access-title">
      <div class="login-brand">
        <img :src="logoMark" :alt="appBranding.appName" class="login-logo" />
      </div>
      <div class="badge">Primeiro acesso</div>
      <h1 id="first-access-title">Crie sua senha definitiva</h1>
      <p class="muted" v-if="isLocalInitialization">
        O usuário master foi preparado internamente. Defina agora a primeira senha da aplicação;
        nenhuma senha temporária precisa ser localizada em arquivo ou no frontend.
      </p>
      <p class="muted" v-else>
        Substitua a senha provisória pela sua senha definitiva para liberar os recursos do perfil.
      </p>

      <div v-if="loadingStatus" class="alert info">Carregando política de segurança...</div>
      <form v-else class="grid" @submit.prevent="submit">
        <div v-if="!isLocalInitialization" class="field">
          <label for="current-password">Senha atual</label>
          <input id="current-password" v-model="form.currentPassword" type="password" autocomplete="current-password" required />
        </div>
        <div class="field">
          <label for="new-password">Nova senha</label>
          <input id="new-password" v-model="form.newPassword" type="password" autocomplete="new-password"
                 :minlength="policy.min_length" :maxlength="policy.max_length" required />
          <small>Requisitos: {{ requirements }}.</small>
        </div>
        <div class="field">
          <label for="password-confirmation">Confirmar nova senha</label>
          <input id="password-confirmation" v-model="form.confirmation" type="password" autocomplete="new-password"
                 :minlength="policy.min_length" :maxlength="policy.max_length" required />
        </div>
        <div v-if="error" class="alert error" role="alert">{{ error }}</div>
        <button class="primary" type="submit" :disabled="session.loading">
          {{ session.loading ? "Salvando..." : "Definir senha e continuar" }}
        </button>
        <button v-if="session.isAuthenticated" class="secondary" type="button" :disabled="session.loading" @click="logout">Sair</button>
      </form>
    </section>
  </main>
</template>

<style scoped>
.first-access-card { max-width: 540px; }
.field small { color: var(--muted, #64748b); line-height: 1.45; }
</style>
