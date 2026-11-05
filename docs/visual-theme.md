# Tema ARGWS Manifestador

O preset `argws-manifestador` é o padrão para usuários sem preferência salva e para a ação explícita de restaurar o padrão. Os 13 presets anteriores continuam disponíveis. Nenhuma migração regrava preferências existentes; modo, densidade, temas personalizados e chaves por usuário são preservados.

| Uso | Cor |
| --- | --- |
| Primária / ação | `#0B4F5C` |
| Ação ao passar o mouse | `#083E49` |
| Acento decorativo | `#00C7B6` |
| Fundo claro | `#F2F6F8` |
| Superfície | `#FFFFFF` |
| Superfície de destaque clara | `#E1F5F2` |
| Superfície de destaque escura | `#153C43` |

O tema preserva a tipografia padrão original da aplicação: `Inter, Segoe UI, Arial, sans-serif`, sem carregar ou impor uma fonte nova. Fontes escolhidas em temas personalizados continuam respeitadas. Escalas e densidades existentes permanecem intactas.

Turquesa não é usado como fundo de botão com texto branco. No modo escuro, o token de texto de destaque usa turquesa sobre superfícies escuras; ações mantêm petróleo e branco. A sidebar clara mantém texto petróleo.

Preferências de aparência continuam pertencendo ao usuário. Esta alteração não move, cria nem modifica configurações fiscais, SMTP ou parâmetros de empresa.

Validação: `npm run test:theme` cobre padrão inicial, preferências existentes, densidades, isolamento por usuário, temas personalizados, alternância e contraste mínimo de 4,5:1 para os pares de texto testados. `npm run build:web` verifica TypeScript e gera o frontend.
