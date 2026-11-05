# Reconstrução do Manifestador NF-e

Esta versão é uma aplicação nova baseada no template Tauri 2 + Rust + Vue 3 + TypeScript. A interface não reproduz as telas Delphi: os recursos foram reorganizados em uma experiência desktop moderna, com empresa ativa, permissões e dados isolados por empresa.

## Recursos migrados

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
