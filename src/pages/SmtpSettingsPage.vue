<script setup lang="ts">
import { computed, onMounted, reactive, ref } from "vue";
import BasePage from "../components/base/BasePage.vue";
import { useSessionStore } from "../stores/session";
import { getGlobalSmtp, saveGlobalSmtp } from "../services/crud";
const session = useSessionStore();
const form = reactive({ enabled: false, smtp_host: "", smtp_conta: "", smtp_email: "", smtp_remetente: "", smtp_porta: 587, smtp_password: "", smtp_password_configured: false });
const busy = ref(false);
const loaded = ref(false);
const error = ref("");
const message = ref("");
const canSave = computed(() => loaded.value && !busy.value && session.can("config:manage"));
onMounted(async () => {
  busy.value = true;
  try { Object.assign(form, await getGlobalSmtp(session.sessionToken!), { smtp_password: "" }); loaded.value = true; }
  catch (err) { error.value = String(err); }
  finally { busy.value = false; }
});
async function save() {
  if (!canSave.value) return;
  busy.value = true; error.value = ""; message.value = "";
  try { Object.assign(form, await saveGlobalSmtp(session.sessionToken!, { ...form }), { smtp_password: "" }); message.value = "SMTP global salvo."; }
  catch (err) { error.value = String(err); }
  finally { busy.value = false; }
}
</script>
<template>
  <BasePage title="SMTP global" subtitle="E-mail da aplicação">
    <p v-if="error" role="alert">{{ error }}</p><p v-if="message" role="status">{{ message }}</p>
    <form class="modal-section-card grid page-gap" @submit.prevent="save">
      <fieldset :disabled="!canSave" class="grid page-gap">
        <label><input v-model="form.enabled" type="checkbox" /> Ativar SMTP global</label>
        <div class="grid columns-2 mobile-columns-1">
          <div class="field"><label for="global-smtp-host">Servidor SMTP</label><input id="global-smtp-host" v-model="form.smtp_host" :required="form.enabled" /></div>
          <div class="field"><label for="global-smtp-port">Porta</label><input id="global-smtp-port" v-model.number="form.smtp_porta" type="number" min="1" max="65535" required /></div>
          <div class="field"><label for="global-smtp-account">Conta</label><input id="global-smtp-account" v-model="form.smtp_conta" autocomplete="off" /></div>
          <div class="field"><label for="global-smtp-email">E-mail do remetente</label><input id="global-smtp-email" v-model="form.smtp_email" type="email" :required="form.enabled" /></div>
          <div class="field"><label for="global-smtp-name">Nome do remetente</label><input id="global-smtp-name" v-model="form.smtp_remetente" /></div>
          <div class="field"><label for="global-smtp-password">Senha {{ form.smtp_password_configured ? "(salva; deixe vazio para preservar)" : "" }}</label><input id="global-smtp-password" v-model="form.smtp_password" type="password" autocomplete="new-password" /></div>
        </div>
      </fieldset>
      <p class="form-hint">As credenciais específicas continuam no cadastro de cada empresa. Salvar não envia e-mails.</p>
      <div class="actions"><button type="submit" :disabled="!canSave">{{ busy ? "Aguarde…" : "Salvar SMTP global" }}</button></div>
    </form>
  </BasePage>
</template>
