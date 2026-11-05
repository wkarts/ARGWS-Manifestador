# Reconstrução do Manifestador NF-e

O Manifestador NF-e organiza documentos fiscais destinados, manifestações e XMLs por empresa, com permissões de consulta e operação.

## Recursos fiscais

- Cadastro e configuração de empresas do aplicativo.
- Migração das tabelas legadas de empresas, notas, itens e histórico para SQLite local.
- Importação de XML completo de NF-e, atualização idempotente por chave/ambiente e preservação dos itens.
- Consulta de notas destinadas e armazenamento de resumos `resNFe` recebidos pela DistribuiçãoDFe.
- Consulta automática configurável por empresa, com NSU persistido.
- Download e cópia local do XML, exportação ZIP e envio do ZIP à contabilidade por SMTP TLS.
- DANFE de consulta, histórico de operações, filtros, manifestação local e integração de manifestação por provedor.
- Integração JSON externa para ACBr/SEFAZ quando a instalação já possuir esse serviço.
- Integração nativa desktop com certificado A1/PFX para DistribuiçãoDFe e download do XML processado.
- API interna e comandos Tauri com autenticação, autorização e isolamento por empresa.

## Integrações fiscais

O certificado A1/PFX é carregado somente no processo desktop e a senha fica protegida no banco local. A consulta nativa usa o endpoint SOAP configurado ou o padrão do ambiente de produção/homologação.

O envio de eventos de manifestação não é falsificado: sem um provedor que faça assinatura XML, transmissão e validação do retorno fiscal, a aplicação registra o evento como local e informa a pendência. Quando o adaptador retorna protocolo, o registro é atualizado como operação enviada.

## Build Windows

Na raiz do projeto, use o wrapper que já aplica o bypass apenas ao processo do build:

```bat
build-windows.bat
```

Também é possível executar o wrapper dentro de `scripts/windows`:

```bat
scripts\windows\build-windows.bat
```

Não é necessário alterar a política global de execução do Windows. O `.bat` chama o PowerShell com `-NoProfile -ExecutionPolicy Bypass -File`.

## Validações locais

```bash
npm run ci:version
npm run typecheck
npm run build:web
python3 scripts/ci/validate-tauri-project.py --project . --mode static
```

As validações Rust (`rustfmt`, Clippy, testes e build Tauri) devem ser executadas no Windows com `build-windows.bat`, pois o ambiente de empacotamento desta reconstrução não inclui o toolchain Rust nativo.


## Parâmetros por empresa e migração

Em **Empresas → Editar → Parâmetros fiscais**, configure consulta, intervalo, SMTP, endpoints, certificado e diretórios do cadastro aberto. O seletor de empresa da barra superior não altera o destino desse formulário. SMTP e dados fiscais são salvos na mesma transação.

A migração cria `manifestador_empresa_operacao` e copia uma única vez os parâmetros anteriormente compartilhados para as empresas já cadastradas. A tabela original é preservada para compatibilidade histórica, mas deixa de alimentar as operações. Alterar uma empresa não atualiza as outras. Empresas novas começam com parâmetros próprios e sem SMTP ou endpoints herdados. Senhas existentes são preservadas quando o campo fica vazio.

O agendador verifica a cada minuto quais empresas estão habilitadas e respeita o intervalo individual, persistindo também as tentativas sem sucesso. Consulta nativa, adaptador e envio à contabilidade leem exclusivamente os parâmetros da empresa da operação.

Os comandos `manifestador_config_get`, `manifestador_config_save` e `manifestador_provider_test` passam a exigir `empresa_id`. Clientes da API devem enviá-lo; vínculo e permissões continuam validados no backend.

## Navegação e indicadores

A entrada principal é a Visão fiscal, seguida de Manifestação NF-e. Cadastros genéricos e ficha técnica não fazem parte do fluxo fiscal e ficam desabilitados na configuração do produto e no acesso às rotas. Suas tabelas e implementações permanecem preservadas; notas e itens fiscais possuem tabelas próprias, sem dependência desses cadastros. Empresas, usuários, perfis, integrações, sincronização, serviços, segurança e licenciamento mantêm suas configurações anteriores.

Os indicadores vêm do resumo fiscal persistido para a empresa selecionada. O painel não consulta a SEFAZ nem trata recurso habilitado como serviço saudável. Registros locais de manifestação não equivalem a eventos autorizados; o histórico e o protocolo devem ser conferidos.
