<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useRouter } from "vue-router";
import { getManifestadorSummary, type GenericRecord } from "../services/crud";
import { useSessionStore } from "../stores/session";
import DashboardMetricCard from "../components/dashboard/DashboardMetricCard.vue";
import AppPageTitleBar from "../components/base/AppPageTitleBar.vue";
const session = useSessionStore();
const router = useRouter();
const empresaId = computed(() => Number(session.activeCompanyId) || null);
const canView = computed(() => session.can("manifestador:view"));
const loading = ref(false);
const stats = ref<GenericRecord | null>(null);
const error = ref("");
let requestId = 0;
async function load() {
  const request = ++requestId;
  const company = empresaId.value;
  stats.value = null;
  error.value = "";
  loading.value = false;
  if (!company || !session.sessionToken || !canView.value) return;
  loading.value = true;
  try {
    const result = await getManifestadorSummary(session.sessionToken, company);
    if (request === requestId) stats.value = result;
  } catch (err) {
    if (request === requestId) error.value = err instanceof Error ? err.message : String(err);
  } finally {
    if (request === requestId) loading.value = false;
  }
}
watch([empresaId, () => session.sessionToken, canView], load, { immediate: true });
</script>
<template>
  <div class="page-content-scroll dashboard-page">
    <AppPageTitleBar title="Visão fiscal" :subtitle="empresaId ? session.activeCompanyName : 'Selecione uma empresa para acompanhar suas NF-e.'" eyebrow="Manifestador NF-e" icon="file">
      <template #actions><button class="secondary" :disabled="loading || !empresaId || !canView" @click="load">{{ loading ? 'Atualizando...' : 'Atualizar' }}</button></template>
    </AppPageTitleBar>
    <p v-if="!canView" role="status">Seu perfil não possui permissão para consultar NF-e. Solicite acesso ao administrador.</p>
    <p v-else-if="!empresaId" class="card" role="status">Escolha a empresa no seletor superior. Os indicadores serão carregados a partir dos documentos dessa empresa.</p>
    <p v-if="loading" role="status">Carregando indicadores fiscais...</p>
    <div v-if="error" class="alert error" role="alert">{{ error }}</div>
    <template v-if="stats">
      <section class="dashboard-metrics-grid fiscal-metrics">
        <DashboardMetricCard title="NF-e registradas" :value="Number(stats.total)" subtitle="Documentos desta empresa" icon="file" status="info" />
        <DashboardMetricCard title="Sem manifestação registrada" :value="Number(stats.pending)" subtitle="Ainda sem registro de evento" icon="clipboard" status="warning" />
        <DashboardMetricCard title="Com registro de manifestação" :value="Number(stats.manifested)" subtitle="Confira o protocolo no histórico" icon="file" status="neutral" />
        <DashboardMetricCard title="XML baixado" :value="Number(stats.downloaded)" subtitle="Downloads registrados" icon="file" status="info" />
      </section>
      <p v-if="Number(stats.total) === 0" class="card" role="status">Nenhuma NF-e registrada para esta empresa. Importe um XML ou configure a consulta para obter documentos.</p>
      <p class="muted-text">Indicadores da base de documentos, sem consulta automática à SEFAZ nesta tela. Um registro local de manifestação não comprova autorização; consulte o retorno e o protocolo no histórico da nota.</p>
    </template>
    <section class="card grid page-gap">
      <h3>Rotina fiscal</h3>
      <p>Consulte notas destinadas, importe XMLs, acompanhe manifestações e prepare os documentos para a contabilidade.</p>
      <div class="actions">
        <button v-if="canView" class="primary" @click="router.push('/manifestador')">Abrir notas fiscais</button>
        <button v-if="session.can('empresas:view')" class="secondary" @click="router.push({ path: '/empresas', query: empresaId ? { empresa: empresaId } : {} })">Empresas e parâmetros fiscais</button>
        <button class="secondary" @click="router.push('/documentacao/guia')">Guia do usuário</button>
      </div>
    </section>
  </div>
</template>

<style scoped>
.fiscal-metrics { grid-template-columns: repeat(auto-fit, minmax(min(100%, 220px), 1fr)); }
</style>
