<script setup lang="ts">
import { isTauriRuntime } from "../core/runtime/RuntimeProvider";
import { computed, reactive, ref, watch } from "vue";
import { useSessionStore } from "../stores/session";
import { importCompanyCertificate, listInstalledCertificates, getManifestadorConfig, getManifestadorCompanyConfig, saveManifestadorCompanyConfig, testManifestadorProvider } from "../services/crud";
const props = defineProps<{ empresaId: number }>();
const session = useSessionStore();
const canManage = computed(() => session.can("manifestador:manage"));
const loading = ref(false);
const saving = ref(false);
const error = ref("");
const message = ref("");
const loaded = ref(false);
function defaultConfig() {
  return {
    verificar_automatico: true,
    download_automatico: true,
    timer_minutos: 60,
    smtp_host: "",
    smtp_conta: "",
    smtp_remetente: "",
    smtp_email: "",
    smtp_password: "",
    smtp_password_configured: false,
    smtp_porta: 587,
    sefaz_endpoint: "",
    sefaz_service_url: "",
    sefaz_timeout_seconds: 60,
  };
}

function defaultCompanyConfig() {
  return {
    certificado_path: "",
    certificado_password: "",
    uf: "",
    xml_path: "",
    numero_serie: "",
    copia_xml_path: "",
    resposta_path: "",
    ambiente: "production",
    nsu: "0",
    data_manifestacao: "",
    certificado_password_configured: false,
  };
}

const config = reactive(defaultConfig());
const companyConfig = reactive(defaultCompanyConfig());

const installedCertificates = ref<{ thumbprint: string; name: string; valid_now: boolean }[]>([]);
const selectedThumbprint = ref("");
const isInstalled = computed(() => companyConfig.certificado_path.startsWith("windows-store://"));
const canSelectInstalled = isTauriRuntime() && /Windows/i.test(navigator.userAgent);
async function pickCertificate(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file || !canManage.value || !loaded.value) return;
  const company = props.empresaId;
  saving.value = true; error.value = "";
  try {
    if (file.size > 1024 * 1024) throw new Error("O certificado deve ter até 1 MiB.");
    const bytes = new Uint8Array(await file.arrayBuffer());
    let binary = "";
    for (let i = 0; i < bytes.length; i += 8192) binary += String.fromCharCode(...bytes.subarray(i, i + 8192));
    const result = await importCompanyCertificate(session.sessionToken!, company, file.name, btoa(binary));
    bytes.fill(0); binary = "";
    if (company !== props.empresaId) return;
    companyConfig.certificado_path = String(result.path);
    companyConfig.certificado_password = "";
    message.value = String(result.message);
  } catch (err) { if (company === props.empresaId) error.value = String(err); }
  finally { input.value = ""; saving.value = false; }
}
async function findInstalled() {
  const company = props.empresaId; saving.value = true; error.value = "";
  try {
    const result = await listInstalledCertificates(session.sessionToken!, company);
    if (company !== props.empresaId) return;
    installedCertificates.value = result.certificates as typeof installedCertificates.value;
    message.value = installedCertificates.value.length ? String(result.message) : "Nenhum certificado com chave privada foi encontrado no usuário Windows atual.";
  } catch (err) { if (company === props.empresaId) error.value = String(err); }
  finally { saving.value = false; }
}
function useInstalled() {
  if (!installedCertificates.value.some(c => c.thumbprint === selectedThumbprint.value && c.valid_now)) return;
  companyConfig.certificado_path = `windows-store://${selectedThumbprint.value}`;
  companyConfig.certificado_password = "";
  message.value = "Certificado selecionado. Salve a empresa. O PIN, quando necessário, será solicitado pelo Windows; consultas com esse certificado são manuais.";
}

let requestId = 0;
watch(() => props.empresaId, async (empresaId) => {
  const request = ++requestId;
  installedCertificates.value = []; selectedThumbprint.value = "";
  Object.assign(config, defaultConfig());
  Object.assign(companyConfig, defaultCompanyConfig());
  error.value = "";
  message.value = "";
  loading.value = true;
  loaded.value = false;
  try {
    const [operational, fiscal] = await Promise.all([getManifestadorConfig(session.sessionToken!, empresaId), getManifestadorCompanyConfig(session.sessionToken!, empresaId)]);
    if (request !== requestId) return;
    Object.assign(config, operational, { smtp_password: "", verificar_automatico: Boolean(Number(operational.verificar_automatico)), download_automatico: Boolean(Number(operational.download_automatico)) });
    Object.assign(companyConfig, fiscal, { certificado_password: "" });
    loaded.value = true;
  } catch (err) {
    if (request === requestId) error.value = String(err);
  } finally {
    if (request === requestId) loading.value = false;
  }
}, { immediate: true });
async function saveConfigForms() {
  const empresaId = props.empresaId;
  saving.value = true;
  error.value = "";
  message.value = "";
  try {
    const { nsu: _nsu, data_manifestacao: _lastQuery, ...fiscal } = companyConfig;
    await saveManifestadorCompanyConfig(session.sessionToken!, empresaId, { ...fiscal, operation_config: { ...config } });
    if (empresaId !== props.empresaId) return;
    config.smtp_password_configured ||= Boolean(config.smtp_password);
    companyConfig.certificado_password_configured ||= Boolean(companyConfig.certificado_password);
    config.smtp_password = "";
    companyConfig.certificado_password = "";
    message.value = "Parâmetros fiscais desta empresa salvos.";
  } catch (err) { if (empresaId === props.empresaId) error.value = String(err); }
  finally { saving.value = false; }
}
async function testProvider() {
  const empresaId = props.empresaId;
  try {
    const result = await testManifestadorProvider(session.sessionToken!, empresaId);
    if (empresaId === props.empresaId) message.value = String(result.message);
  } catch (err) { if (empresaId === props.empresaId) error.value = String(err); }
}
</script>
<template>
<section class="grid page-gap">
  <h3>Parâmetros fiscais desta empresa</h3>
  <p>Certificado, consulta, diretórios e SMTP pertencem exclusivamente ao cadastro aberto.</p>
  <p v-if="loading" role="status">Carregando parâmetros...</p>
  <div v-if="error" class="alert error" role="alert">{{ error }}</div>
  <p v-if="message" role="status">{{ message }}</p>
  <fieldset :disabled="loading || saving || !canManage || !loaded" style="border: 0; padding: 0; min-width: 0">
      <div class="grid page-gap">
        <div class="grid columns-2 mobile-columns-1">
          <section class="modal-section-card grid page-gap"><div class="section-title">Consulta SEFAZ e adaptador externo</div><label class="checkbox-inline"><input v-model="config.verificar_automatico" type="checkbox" /> Consultar automaticamente</label><label class="checkbox-inline"><input v-model="config.download_automatico" type="checkbox" /> Baixar XML automaticamente</label><div class="field"><label>Intervalo (minutos)</label><input v-model.number="config.timer_minutos" type="number" min="1" /></div><div class="field"><label>Endpoint JSON externo (opcional)</label><input v-model="config.sefaz_endpoint" type="url" placeholder="https://servidor.local/manifestador" /></div><div class="field"><label>Endpoint SOAP DistribuiçãoDFe (opcional)</label><input v-model="config.sefaz_service_url" type="url" placeholder="Padrão nacional conforme ambiente" /></div><div class="field"><label>Timeout (segundos)</label><input v-model.number="config.sefaz_timeout_seconds" type="number" min="5" max="300" /></div><button class="secondary" type="button" @click="testProvider" :disabled="loading || saving">Ver configuração salva do endpoint</button><div class="form-hint">Sem endpoint JSON, o aplicativo desktop usa o certificado A1/PFX e o endpoint SOAP configurado ou padrão do ambiente.</div></section>
          <section class="modal-section-card grid page-gap"><div class="section-title">SMTP para contabilidade</div><div class="field"><label>Servidor SMTP</label><input v-model="config.smtp_host" type="text" /></div><div class="grid columns-2 mobile-columns-1"><div class="field"><label>Conta</label><input v-model="config.smtp_conta" type="text" /></div><div class="field"><label>Porta</label><input v-model.number="config.smtp_porta" type="number" min="1" /></div></div><div class="field"><label>Remetente</label><input v-model="config.smtp_remetente" type="text" /></div><div class="field"><label>E-mail do remetente</label><input v-model="config.smtp_email" type="email" /></div><div class="field"><label>Senha SMTP {{ config.smtp_password_configured ? "(já configurada; informe somente para substituir)" : "" }}</label><input v-model="config.smtp_password" type="password" autocomplete="new-password" /></div><div class="form-hint">A senha é armazenada protegida localmente. O envio usa SMTP com TLS no desktop. A exportação ZIP também está disponível.</div></section>
        </div>
        <section class="modal-section-card grid page-gap"><div class="section-title">Certificado e diretórios da empresa deste cadastro</div><div class="grid columns-2 mobile-columns-1"><div class="field"><label>Certificado A1 por arquivo</label><input type="file" accept=".pfx,.p12,application/x-pkcs12" aria-label="Selecionar certificado PFX ou P12" @change="pickCertificate" /><label>Arquivo salvo ou referência do certificado instalado</label><input v-model="companyConfig.certificado_path" type="text" /></div>
          <div class="field"><label>Certificados instalados (A1/A3)</label><button type="button" class="secondary" :disabled="!canSelectInstalled" @click="findInstalled">Buscar no Windows</button><p class="form-hint">Disponível no aplicativo Windows. A3 depende do dispositivo e do driver do fabricante. A chave privada não é exportada.</p><template v-if="installedCertificates.length"><select v-model="selectedThumbprint" aria-label="Certificado instalado"><option value="">Selecione um certificado</option><option v-for="cert in installedCertificates" :key="cert.thumbprint" :value="cert.thumbprint" :disabled="!cert.valid_now">{{ cert.name }} — {{ cert.thumbprint.slice(-8) }}{{ cert.valid_now ? "" : " (fora da validade)" }}</option></select><button type="button" class="secondary" :disabled="!selectedThumbprint" @click="useInstalled">Usar certificado selecionado</button></template></div><div class="field"><label>Senha do certificado {{ companyConfig.certificado_password_configured ? "(já configurada)" : "" }}</label><input v-model="companyConfig.certificado_password" :disabled="isInstalled" type="password" autocomplete="new-password" /></div><div class="field"><label>UF</label><input v-model="companyConfig.uf" maxlength="2" /></div><div class="field"><label>Ambiente</label><select v-model="companyConfig.ambiente"><option value="production">Produção</option><option value="homologation">Homologação</option></select></div><div class="field"><label>Diretório de XML</label><input v-model="companyConfig.xml_path" type="text" /></div><div class="field"><label>Cópia de XML</label><input v-model="companyConfig.copia_xml_path" type="text" /></div><div class="field"><label>Diretório de respostas</label><input v-model="companyConfig.resposta_path" type="text" /></div><div class="field"><label>Número de série</label><input v-model="companyConfig.numero_serie" type="text" /></div></div><div class="form-hint">PFX/P12 é usado no desktop para DistribuiçãoDFe e download. No Windows, certificados instalados usam o provedor CSP/CNG do sistema, sem armazenar PIN. A consulta com certificado instalado é manual. No navegador, o arquivo é copiado para o servidor conectado; não é possível acessar o A3 do computador pelo servidor. A manifestação de eventos continua exigindo assinatura/evento compatível com o provedor configurado; sem provedor, o histórico local é preservado e nada é falsamente enviado.</div></section>
        <div class="actions modal-actions-footer"><button class="primary" type="button" :disabled="loading || saving || !canManage" @click="saveConfigForms">{{ saving ? "Salvando..." : "Salvar configurações" }}</button></div>
      </div>

  </fieldset>
</section>
</template>
