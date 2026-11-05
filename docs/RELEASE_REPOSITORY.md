# Release e repositório GitHub

O template não deve apontar para um repositório fixo.

A configuração de release usa esta ordem:

1. `REPOSITORY_URL`, quando definida explicitamente;
2. `GITHUB_SERVER_URL` + `GITHUB_REPOSITORY`, no GitHub Actions;
3. autodetecção padrão do `semantic-release`, quando executado localmente.

Isso evita erro como:

```text
remote: Repository not found.
fatal: repository 'https://github.com/<org>/<repo>.git/' not found
```

## Exemplo opcional

```yaml
env:
  REPOSITORY_URL: https://github.com/sua-org/seu-repositorio.git
```

Na maioria dos repositórios GitHub, não é necessário configurar `REPOSITORY_URL`.

## Canais de entrega do Manifestador NF-e

O workflow `Release` acompanha o CI aprovado de pushes de `main` e `develop` do próprio repositório. PRs não publicam releases. O preparo usa `scripts/release/prepare-ci.mjs`; `release.config.mjs` e o comando semantic-release antigo não são o caminho de publicação desse workflow.

- `develop`: versão numérica `X.Y.Z`, tag `vX.Y.Z`, release com `prerelease=true` e `latest=false`.
- `main`: versão numérica e release estável, sem reutilizar uma tag de pré-release.
- Conventional Commits definem patch/minor/major a partir da última versão estável reservada. A versão declarada no produto também pode estabelecer um piso maior. Sem incremento indicado, o padrão é patch.
- Todas as tags numéricas existentes, inclusive drafts e pré-releases, reservam a versão. A próxima versão precisa superar a maior reserva; tags históricas sem metadados são tratadas conservadoramente como estáveis.
- Exemplo: `v1.0.0` reservada → develop `v1.0.1` (pré-release) → outro commit `v1.0.2` → main `v1.0.3` estável. Não há promoção que mova ou sobrescreva a tag anterior. Versões numéricas usadas em pré-releases ficam consumidas.

O preparo sincroniza VERSION, npm, Cargo/lockfile, Tauri e configuração do frontend em um commit destacado, com origem no SHA aprovado pelo CI. Publica somente a tag anotada, sem commit automático na branch. A anotação registra SHA de origem e canal. A matriz existente de desktop, Web/PWA, CloudPanel, Android e iOS permanece a mesma.

Uma repetição do mesmo SHA/canal reutiliza a tag imutável: se publicada, é um no-op; se draft, retoma o pipeline. Assets existentes são comparados por hash e nunca substituídos. Um rebuild não determinístico com hash diferente é recusado e exige uma nova versão, em vez de sobrescrever arquivos. Erros de autenticação/rede não são tratados como ausência de release. Commits de CI que já não são a ponta da branch não geram versão.

A publicação só ocorre após matriz completa, salvo a autorização manual de publicação parcial que já existia. Retomar uma tag preserva o indicador de pré-release mesmo sem sufixo na versão. O grupo único de concorrência serializa os dois canais; uma colisão externa de tag faz o push falhar sem force-push.

**Ativação:** `workflow_run` usa a definição da branch padrão (`main`). Portanto, incluir `develop` no gatilho só entra em vigor depois que a mudança do workflow chegar à branch padrão. Preparar a PR não ativa nem dispara uma release. A `v1.0.0` atualmente em draft não é reaberta ou publicada por esta alteração.

**Verificação:** `node --test scripts/release/tests/*.test.mjs` cobre planejamento SemVer, canais, colisões e repetição com Git local e API simulada. Não executa publicação real. Para rollback, reverta os arquivos do workflow/preparo em nova PR; preserve todas as tags/releases já reservadas. Não há migração de banco nem mudança na interface.

Referência consultada: `wkarts/ARGWS-Scale-Simulator`, workflow Release e scripts `prepare-ci.mjs`/`version-plan.mjs`. Esse projeto usa `-beta`; o Manifestador usa o indicador de pré-release do GitHub para cumprir a exigência de versão sem sufixo. ADP não foi localizado nas integrações disponíveis.
