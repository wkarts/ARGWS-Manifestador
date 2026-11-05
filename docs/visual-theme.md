# Tema ARGWS Manifestador

O preset `argws-manifestador` é o padrão para usuários sem preferência salva e para a ação explícita de restaurar o padrão. Os 13 presets anteriores continuam disponíveis. Nenhuma migração regrava preferências existentes; modo, densidade, temas personalizados e chaves por usuário são preservados.

| Uso | Cor |
| --- | --- |
| Primária / ação | `#334E7D` |
| Ação ao passar o mouse | `#263B60` |
| Acento decorativo | `#64748B` |
| Fundo claro | `#F5F6F8` |
| Superfície | `#FFFFFF` |
| Superfície de destaque clara | `#EDF1F7` |
| Superfície de destaque escura | `#243247` |

O tema preserva a tipografia padrão original da aplicação: `Inter, Segoe UI, Arial, sans-serif`, sem carregar ou impor uma fonte nova. Fontes escolhidas em temas personalizados continuam respeitadas. Escalas e densidades existentes permanecem intactas.

O azul acinzentado concentra os destaques nas ações. Fundos e cartões usam cinza neutro e branco. No modo escuro, o texto de destaque usa azul claro (`#AFC6EE`); botões mantêm azul sóbrio e branco. A sidebar clara mantém texto azul sóbrio.

Preferências de aparência continuam pertencendo ao usuário. Esta alteração não move, cria nem modifica configurações fiscais, SMTP ou parâmetros de empresa.

Validação: `npm run test:theme` cobre padrão inicial, preferências existentes, densidades, isolamento por usuário, temas personalizados, alternância e contraste mínimo de 4,5:1 para os pares de texto testados. `npm run build:web` verifica TypeScript e gera o frontend.
