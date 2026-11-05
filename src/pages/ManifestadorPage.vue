<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { useRouter } from "vue-router";
import AppModal from "../components/AppModal.vue";
import BasePage from "../components/base/BasePage.vue";
import { useSessionStore } from "../stores/session";
import {
  downloadManifestadorNote,
  exportManifestadorZip,
  fetchManifestadorNotes,
  getManifestadorCompanyConfig,
  getManifestadorConfig,
  processFiscalNote,
  getFiscalEngineStatus,
  getManifestadorSummary,
  importManifestadorXml,
  listManifestadorNoteLogs,
  listManifestadorNotes,
  manifestManifestadorNote,
  sendManifestadorAccounting,
  type GenericRecord,
} from "../services/crud";
import { showSplashInfo, showSplashSuccess, showSplashWarning } from "../services/splash";


const session = useSessionStore();
const router = useRouter();
const loading = ref(false);
const saving = ref(false);
const error = ref("");
const search = ref("");
const statusFilter = ref("");
const rows = ref<GenericRecord[]>([]);
const summary = ref<GenericRecord>({ total: 0, downloaded: 0, manifested: 0, pending: 0 });
const importModalOpen = ref(false);
const manifestModalOpen = ref(false);
const logsModalOpen = ref(false);
const accountingModalOpen = ref(false);
const xmlDraft = ref("");
const selectedNote = ref<GenericRecord | null>(null);
const logs = ref<GenericRecord[]>([]);
const accountingForm = reactive({ dataInicial: "", dataFinal: "", destinatario: "" });

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
const manifestForm = reactive({ evento: "ciencia", justificativa: "" });
const currentCompanyId = computed(() => session.activeCompanyId ? Number(session.activeCompanyId) : null);
const canManage = computed(() => session.can("manifestador:manage"));
const providerConfigured = computed(() => Boolean(String(config.sefaz_endpoint || "").trim() || String(config.sefaz_service_url || "").trim() || (String(companyConfig.certificado_path || "").trim() && String(companyConfig.uf || "").trim())));

function text(value: unknown, fallback = "") {
  return value == null ? fallback : String(value);
}

function number(value: unknown) {
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : 0;
}

function money(value: unknown) {
  return number(value).toLocaleString("pt-BR", { style: "currency", currency: "BRL" });
}

function dateTime(value: unknown) {
  const raw = text(value);
  return raw ? raw.replace("T", " ").slice(0, 19) : "-";
}

function isoDate(date: Date) {
  return date.toISOString().slice(0, 10);
}

function defaultAccountingPeriod() {
  const now = new Date();
  const firstCurrentMonth = new Date(now.getFullYear(), now.getMonth(), 1);
  const lastPreviousMonth = new Date(firstCurrentMonth.getTime() - 24 * 60 * 60 * 1000);
  const firstPreviousMonth = new Date(lastPreviousMonth.getFullYear(), lastPreviousMonth.getMonth(), 1);
  accountingForm.dataInicial = isoDate(firstPreviousMonth);
  accountingForm.dataFinal = isoDate(lastPreviousMonth);
  accountingForm.destinatario = "";
}



function statusLabel(row: GenericRecord) {
  const manifestation = text(row.manifestacao, "N");
  if (manifestation === "0") return "Confirmada";
  if (manifestation === "1") return "Ciência";
  if (manifestation === "2") return "Não realizada";
  if (manifestation === "3") return "Desconhecida";
  return text(row.situacao, "Recebida");
}

function resetConfig() {
  Object.assign(config, defaultConfig());
}

function resetCompanyConfig() {
  Object.assign(companyConfig, defaultCompanyConfig());
}

let requestId = 0;
async function load() {
  const request = ++requestId;
  const empresaId = currentCompanyId.value;
  rows.value = [];
  summary.value = {};
  resetConfig();
  resetCompanyConfig();
  error.value = "";
  loading.value = false;
  if (!session.sessionToken || !empresaId) return;
  loading.value = true;
  try {
    const [list, stats, operational, fiscal] = await Promise.all([
      listManifestadorNotes(session.sessionToken, { search: search.value, status: statusFilter.value, empresa_id: empresaId }),
      getManifestadorSummary(session.sessionToken, empresaId),
      getManifestadorConfig(session.sessionToken, empresaId),
      getManifestadorCompanyConfig(session.sessionToken, empresaId),
    ]);
    if (request !== requestId) return;
    rows.value = list;
    summary.value = stats;
    Object.assign(config, operational, { smtp_password: "" });
    Object.assign(companyConfig, fiscal, { certificado_password: "" });
  } catch (err) {
    if (request === requestId) error.value = err instanceof Error ? err.message : "Falha ao carregar o Manifestador.";
  } finally {
    if (request === requestId) loading.value = false;
  }
}

async function refresh() {
  await load();
  showSplashInfo("Dados do Manifestador atualizados.");
}

async function obtainNotes() {
  if (!session.sessionToken || !currentCompanyId.value) {
    showSplashWarning("Selecione uma empresa ativa antes de consultar NF-e.");
    return;
  }
  saving.value = true;
  try {
    const result = await fetchManifestadorNotes(session.sessionToken, currentCompanyId.value);
    if (result.ok === false) showSplashWarning(text(result.message));
    else showSplashSuccess(text(result.message, "Consulta concluída."));
    await load();
  } finally {
    saving.value = false;
  }
}

function openImportModal() {
  xmlDraft.value = "";
  importModalOpen.value = true;
}

async function readXmlFile(event: Event) {
  const file = (event.target as HTMLInputElement).files?.[0];
  if (!file) return;
  xmlDraft.value = await file.text();
}

async function importXml() {
  if (!session.sessionToken || !currentCompanyId.value || !xmlDraft.value.trim()) {
    showSplashWarning("Selecione uma empresa e informe um XML completo de NF-e.");
    return;
  }
  saving.value = true;
  try {
    await importManifestadorXml(session.sessionToken, currentCompanyId.value, xmlDraft.value);
    importModalOpen.value = false;
    showSplashSuccess("XML importado e seus itens/histórico localizados.");
    await load();
  } finally {
    saving.value = false;
  }
}

async function exportZip() {
  if (!session.sessionToken) return;
  const result = await exportManifestadorZip(session.sessionToken, currentCompanyId.value, { status: statusFilter.value });
  showSplashSuccess(`${text(result.message, "ZIP gerado.")} Caminho: ${text(result.path)}`);
}

async function download(row: GenericRecord) {
  if (!session.sessionToken || !currentCompanyId.value) return;
  const result = await downloadManifestadorNote(session.sessionToken, currentCompanyId.value, number(row.id));
  showSplashSuccess(text(result.message, `XML salvo em ${text(result.path)}`));
  await load();
}

async function fiscal(row: GenericRecord, operation: "validate" | "danfe") {
  if (!session.sessionToken || !currentCompanyId.value || saving.value) return;
  const company = currentCompanyId.value;
  saving.value = true;
  try {
    const result = await processFiscalNote(session.sessionToken, company, number(row.id), operation);
    if (company !== currentCompanyId.value) return;
    if (operation === "validate") { showSplashSuccess(String(result.message)); return; }
    const bytes = Uint8Array.from(atob(String(result.content_base64)), c => c.charCodeAt(0));
    const url = URL.createObjectURL(new Blob([bytes], { type: "application/pdf" }));
    const link = document.createElement("a");
    link.href = url; link.download = String(result.file_name);
    document.body.appendChild(link); link.click(); link.remove();
    setTimeout(() => URL.revokeObjectURL(url), 60_000);
    showSplashSuccess(`DANFE PDF gerado pela ACBrLib e salvo no servidor: ${String(result.path)}. A situação atual na SEFAZ não foi consultada.`);
  } catch (err) { showSplashWarning(err instanceof Error ? err.message : String(err)); }
  finally { saving.value = false; }
}

const fiscalStatus = ref<GenericRecord>({ message: "Verificando disponibilidade do motor fiscal…" });
onMounted(async () => {
  if (!session.sessionToken) return;
  try { fiscalStatus.value = await getFiscalEngineStatus(session.sessionToken); }
  catch (err) { fiscalStatus.value = { message: String(err) }; }
});

function openManifest(row: GenericRecord) {
  selectedNote.value = row;
  manifestForm.evento = "ciencia";
  manifestForm.justificativa = "";
  manifestModalOpen.value = true;
}

async function submitManifest() {
  if (!session.sessionToken || !currentCompanyId.value || !selectedNote.value) return;
  saving.value = true;
  try {
    const result = await manifestManifestadorNote(
      session.sessionToken,
      currentCompanyId.value,
      number(selectedNote.value.id),
      manifestForm.evento,
      manifestForm.justificativa,
    );
    manifestModalOpen.value = false;
    if (result.warning) showSplashWarning(text(result.warning));
    else showSplashSuccess(text(result.message, "Manifestação registrada."));
    await load();
  } finally {
    saving.value = false;
  }
}

async function openLogs(row: GenericRecord) {
  if (!session.sessionToken || !currentCompanyId.value) return;
  selectedNote.value = row;
  logs.value = await listManifestadorNoteLogs(session.sessionToken, currentCompanyId.value, number(row.id));
  logsModalOpen.value = true;
}

function openAccountingModal() {
  defaultAccountingPeriod();
  accountingModalOpen.value = true;
}

async function sendAccounting() {
  if (!session.sessionToken || !currentCompanyId.value) return;
  if (!accountingForm.dataInicial || !accountingForm.dataFinal || !accountingForm.destinatario.trim()) {
    showSplashWarning("Informe o período e o e-mail da contabilidade.");
    return;
  }
  saving.value = true;
  try {
    const result = await sendManifestadorAccounting(
      session.sessionToken,
      currentCompanyId.value,
      accountingForm.dataInicial,
      `${accountingForm.dataFinal}T23:59:59`,
      accountingForm.destinatario.trim(),
    );
    accountingModalOpen.value = false;
    showSplashSuccess(text(result.message, "XMLs enviados à contabilidade."));
  } catch (err) {
    showSplashWarning(err instanceof Error ? err.message : "Falha ao enviar XMLs à contabilidade.");
  } finally {
    saving.value = false;
  }
}

function openConfig() {
  if (currentCompanyId.value) void router.push({ path: "/empresas", query: { empresa: currentCompanyId.value } });
}

watch(currentCompanyId, () => {
  manifestModalOpen.value = false;
  logsModalOpen.value = false;
  accountingModalOpen.value = false;
  importModalOpen.value = false;
  selectedNote.value = null;
  logs.value = [];
  void load();
});
onMounted(load);
</script>

<template>
  <BasePage title="Manifestação de NF-e" subtitle="Distribuição, consulta, manifestação, download e organização de NF-e destinadas." icon="file">
    <p class="form-hint" role="status">{{ fiscalStatus.message }}</p>
    <template #actions>
      <button v-if="session.can('empresas:view')" class="secondary" type="button" :disabled="!currentCompanyId" @click="openConfig">Cadastro e parâmetros da empresa</button>
      <button class="secondary" type="button" :disabled="loading" @click="refresh">Atualizar</button>
      <button class="primary" type="button" :disabled="saving || !canManage" @click="obtainNotes">Obter notas</button>
    </template>

    <p v-if="!currentCompanyId" role="status">Selecione uma empresa no seletor superior para consultar suas NF-e.</p>
    <div v-if="error" class="alert error">{{ error }}</div>
    <div v-if="!currentCompanyId" class="alert info">Selecione uma empresa ativa no topo para operar o Manifestador.</div>

    <div class="kpis">
      <div class="kpi"><strong>Total de NF-e</strong><span>{{ summary.total || 0 }}</span></div>
      <div class="kpi"><strong>Pendentes</strong><span>{{ summary.pending || 0 }}</span></div>
      <div class="kpi"><strong>Com registro de manifestação</strong><span>{{ summary.manifested || 0 }}</span></div>
      <div class="kpi"><strong>XML baixado</strong><span>{{ summary.downloaded || 0 }}</span></div>
    </div>

    <div class="card grid page-gap">
      <div class="toolbar">
        <div>
          <h3>Notas fiscais destinadas</h3>
          <p class="muted">O isolamento segue a empresa ativa e os vínculos do usuário autenticado.</p>
        </div>
        <div class="actions align-end">
          <div class="field min-field"><label>Buscar</label><input v-model="search" type="search" placeholder="Chave, emitente ou número" @keyup.enter="load" /></div>
          <div class="field"><label>Situação</label><select v-model="statusFilter" @change="load"><option value="">Todas</option><option value="recebida">Recebida</option><option value="baixada">Baixada</option><option value="manifestada_local">Manifestada</option></select></div>
          <button class="secondary" type="button" @click="openImportModal">Importar XML</button>
          <button class="secondary" type="button" :disabled="!canManage || !currentCompanyId" @click="openAccountingModal">Enviar contabilidade</button>
          <button class="secondary" type="button" :disabled="!canManage" @click="exportZip">Exportar ZIP</button>
        </div>
      </div>

      <div v-if="providerConfigured" class="form-hint">Integração SEFAZ configurada. No desktop, a consulta e o download usam o certificado A1/PFX nativamente quando não houver endpoint JSON externo; a operação local continua disponível.</div>
      <div v-else class="form-hint">Configure o certificado A1/PFX e a UF da empresa para consultar a SEFAZ nativamente, ou informe um endpoint JSON externo compatível. A importação local permanece disponível.</div>

      <div class="table-wrap">
        <table>
          <thead><tr><th>NF-e</th><th>Emitente</th><th>Emissão</th><th>Valor</th><th>Manifestação</th><th>Download</th><th>NSU</th><th>Ações</th></tr></thead>
          <tbody>
            <tr v-for="row in rows" :key="number(row.id)">
              <td><strong>{{ row.numero || "-" }}</strong><div class="muted-row">Série {{ row.serie || "-" }} · {{ row.chave || "sem chave" }}</div></td>
              <td>{{ row.nome || "-" }}<div class="muted-row">{{ row.cnpj || "-" }}</div></td>
              <td>{{ dateTime(row.emissao) }}</td>
              <td>{{ money(row.valor) }}</td>
              <td><span class="status-pill">{{ statusLabel(row) }}</span></td>
              <td>{{ Number(row.download) === 1 ? "Sim" : "Não" }}</td>
              <td>{{ row.nsu || "0" }}</td>
              <td><div class="compact-actions actions"><button class="secondary" type="button" :disabled="saving" @click="fiscal(row, 'danfe')">DANFE PDF</button><button class="secondary" type="button" :disabled="saving" @click="fiscal(row, 'validate')">Validar XSD</button><button class="secondary" type="button" :disabled="!canManage" @click="download(row)">XML</button><button class="secondary" type="button" :disabled="!canManage" @click="openManifest(row)">Manifestar</button><button class="secondary" type="button" @click="openLogs(row)">Histórico</button></div></td>
            </tr>
            <tr v-if="!rows.length"><td colspan="8" class="empty-cell">{{ loading ? "Carregando notas..." : "Nenhuma NF-e encontrada para a empresa ativa." }}</td></tr>
          </tbody>
        </table>
      </div>
    </div>

    <AppModal :open="importModalOpen" title="Importar XML de NF-e" subtitle="A importação atualiza resumo, itens e histórico local." width="xl" @close="importModalOpen = false">
      <div class="grid page-gap">
        <div class="field"><label>Arquivo XML</label><input type="file" accept=".xml,text/xml,application/xml" @change="readXmlFile" /></div>
        <div class="field"><label>Conteúdo XML</label><textarea v-model="xmlDraft" class="textarea-mono" rows="14" placeholder="Cole aqui o XML completo da NF-e ou selecione um arquivo." /></div>
        <div class="actions modal-actions-footer"><button class="secondary" type="button" @click="importModalOpen = false">Cancelar</button><button class="primary" type="button" :disabled="saving || !xmlDraft.trim()" @click="importXml">{{ saving ? "Importando..." : "Importar XML" }}</button></div>
      </div>
    </AppModal>



    <AppModal :open="manifestModalOpen" title="Manifestar NF-e" subtitle="A justificativa é obrigatória para operação não realizada." width="md" @close="manifestModalOpen = false">
      <div class="grid page-gap"><div class="form-hint">Chave: {{ selectedNote?.chave || "-" }}</div><div class="field"><label>Evento</label><select v-model="manifestForm.evento"><option value="ciencia">Ciência da operação</option><option value="confirmacao">Confirmação da operação</option><option value="desconhecimento">Desconhecimento da operação</option><option value="nao_realizada">Operação não realizada</option></select></div><div v-if="manifestForm.evento === 'nao_realizada'" class="field"><label>Justificativa (mínimo 15 caracteres)</label><textarea v-model="manifestForm.justificativa" rows="5" /></div><div class="form-hint">Sem adaptador externo, o evento será marcado localmente para não perder o histórico. O envio fiscal real requer assinatura e retorno da SEFAZ.</div><div class="actions modal-actions-footer"><button class="secondary" type="button" @click="manifestModalOpen = false">Cancelar</button><button class="primary" type="button" :disabled="saving || !canManage" @click="submitManifest">{{ saving ? "Registrando..." : "Registrar manifestação" }}</button></div></div>
    </AppModal>

    <AppModal :open="logsModalOpen" title="Histórico da NF-e" subtitle="Auditoria das consultas, importações, downloads e manifestações." width="lg" @close="logsModalOpen = false">
      <div class="table-wrap"><table><thead><tr><th>Data/hora</th><th>Operação</th><th>Histórico</th></tr></thead><tbody><tr v-for="item in logs" :key="number(item.id)"><td>{{ dateTime(item.datahora) }}</td><td>{{ item.operacao }}</td><td>{{ item.historico }}</td></tr><tr v-if="!logs.length"><td colspan="3" class="empty-cell">Nenhum histórico registrado.</td></tr></tbody></table></div>
    </AppModal>

    <AppModal :open="accountingModalOpen" title="Enviar XMLs à contabilidade" subtitle="Selecione o período e o destinatário. O ZIP será gerado localmente e enviado pelo SMTP configurado." width="md" @close="accountingModalOpen = false">
      <div class="grid page-gap">
        <div class="grid columns-2 mobile-columns-1">
          <div class="field"><label>Data inicial</label><input v-model="accountingForm.dataInicial" type="date" /></div>
          <div class="field"><label>Data final</label><input v-model="accountingForm.dataFinal" type="date" /></div>
        </div>
        <div class="field"><label>E-mail da contabilidade</label><input v-model="accountingForm.destinatario" type="email" placeholder="contabilidade@empresa.com.br" /></div>
        <div class="form-hint">O SMTP deve estar configurado em Configurações. O envio usa TLS implícito na porta 465 ou STARTTLS nas demais portas quando anunciado pelo servidor.</div>
        <div class="actions modal-actions-footer"><button class="secondary" type="button" @click="accountingModalOpen = false">Cancelar</button><button class="primary" type="button" :disabled="saving" @click="sendAccounting">{{ saving ? "Enviando..." : "Gerar e enviar ZIP" }}</button></div>
      </div>
    </AppModal>
  </BasePage>
</template>
