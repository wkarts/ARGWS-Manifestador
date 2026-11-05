<script setup lang="ts">
import BasePage from "../components/base/BasePage.vue";
import BaseSectionCard from "../components/base/BaseSectionCard.vue";
</script>
<template>
  <BasePage title="Guia do Manifestador NF-e" subtitle="Da configuração da empresa à entrega de XMLs à contabilidade.">
    <div class="grid columns-2 mobile-columns-1">
      <BaseSectionCard title="1. Cadastre e selecione a empresa">
        <p>Em Empresas, informe os dados fiscais e salve o cadastro. Abra a empresa para configurar certificado A1/PFX, UF, ambiente, diretórios, consulta e SMTP. Os parâmetros pertencem à empresa aberta no cadastro.</p>
        <p>Selecione a empresa no topo da aplicação antes de trabalhar com suas notas. Confira se o ambiente é produção ou homologação.</p>
      </BaseSectionCard>
      <BaseSectionCard title="2. Obtenha os documentos">
        <p>Em Manifestação NF-e, importe XMLs ou use Obter notas. A consulta nativa usa certificado A1/PFX no desktop; o adaptador externo deve ser configurado quando utilizado. Ter um endpoint preenchido não comprova conectividade.</p>
        <p>O painel fiscal apresenta dados da base da empresa selecionada. Uma lista vazia indica ausência de registros, não ausência de documentos na SEFAZ.</p>
      </BaseSectionCard>
      <BaseSectionCard title="3. Acompanhe as manifestações">
        <p>Abra a nota, confira o evento e a justificativa antes de manifestar. Consulte seu histórico para verificar resposta e protocolo.</p>
        <p>Sem provedor compatível, o sistema registra a solicitação localmente; isso não confirma envio ou autorização pela SEFAZ.</p>
      </BaseSectionCard>
      <BaseSectionCard title="4. Prepare XMLs para a contabilidade">
        <p>Baixe os XMLs completos disponíveis. Exporte um ZIP ou informe período e destinatário em Enviar contabilidade. O envio usa o SMTP da empresa selecionada e depende da disponibilidade do serviço.</p>
      </BaseSectionCard>
      <BaseSectionCard title="Acesso e segurança">
        <p>Troque a senha inicial e vincule usuários às empresas e perfis necessários. Permissões de consulta e operação fiscal são distintas. Senhas de SMTP e certificado não são devolvidas nos formulários; deixe em branco para manter as existentes.</p>
      </BaseSectionCard>
      <BaseSectionCard title="DANFE PDF e validação XSD">
        <p>No menu Manifestador NF-e, baixe o XML completo e use DANFE PDF ou Validar XSD. Esses recursos exigem ACBrLib e schemas instalados no servidor Windows/Linux compatível. A tela informa quando não estão configurados. A validação XSD não verifica a assinatura nem confirma a situação atual na SEFAZ.</p>
      </BaseSectionCard>
      <BaseSectionCard title="Diagnóstico">
        <p>Use o histórico da nota para retornos fiscais e Logs para falhas da aplicação. As áreas de integrações e serviços são administrativas; sua disponibilidade no menu não indica que estejam conectadas.</p>
      </BaseSectionCard>
    </div>
  </BasePage>
</template>
