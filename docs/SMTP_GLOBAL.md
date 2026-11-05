# SMTP global

A tela **Sistema → SMTP global** armazena as credenciais da aplicação separadamente das credenciais de cada empresa. Consulta exige `config:view`; alteração exige `config:manage`. Os comandos são `application_smtp_get` e `application_smtp_save`, disponíveis no desktop e na API autenticada.

A migração acrescenta `application_smtp` desativada e sem credenciais. Não reutiliza automaticamente o antigo registro histórico `manifestador_config`, não altera SMTP empresarial nem dispara e-mail. A senha nunca é devolvida; campo vazio preserva a senha já salva. O armazenamento usa o mecanismo de proteção local já adotado pela aplicação.

**Pendência desta PR em rascunho:** a precedência entre SMTP empresarial e global foi perguntada ao responsável e ainda não foi respondida. Até essa definição, o envio existente continua exclusivamente com o SMTP da empresa. Ativar e salvar o cadastro global ainda não altera a origem das credenciais de envio. Não integrar esta etapa como fluxo SMTP completo antes de resolver essa pendência.

Não houve envio real de e-mail nos testes. A disponibilidade do envio SMTP nativo permanece limitada ao runtime desktop, conforme implementação existente; cadastro acessível via API não representa envio SMTP headless implementado.
