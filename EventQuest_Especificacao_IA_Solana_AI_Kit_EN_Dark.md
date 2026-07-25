# EventQuest — Especificação funcional e técnica para desenvolvimento por IA

> **Versão 2.0 — Solana AI Kit + entrega funcional na Devnet**  
> Documento de produto e implementação para orientar uma IA de desenvolvimento na criação de uma plataforma full stack integrada à blockchain Solana.

---

## 0. Diretriz obrigatória: Solana AI Kit e entrega na Devnet

O desenvolvimento deve utilizar o repositório:

```text
https://github.com/solanabr/solana-ai-kit
```

### 0.1 Papel do Solana AI Kit

O `solana-ai-kit` deve ser tratado como a **camada de configuração, conhecimento, agentes, comandos, regras e ferramentas da IA desenvolvedora**.

Ele não é:

- Um SDK de execução da aplicação;
- Um framework web;
- Um programa Solana;
- Uma biblioteca que será importada pelo frontend em produção;
- Um substituto para Anchor, `@solana/kit`, PostgreSQL ou Redis.

Ele deve orientar a IA no planejamento, implementação, testes, auditoria e deploy do projeto.

A aplicação em produção deve continuar funcionando sem um processo do `solana-ai-kit` em execução.

### 0.2 Forma de instalação obrigatória

Para Claude Code, preferir a instalação completa no diretório raiz do projeto, pois ela inclui:

- Agentes;
- Comandos;
- Regras carregadas por tipo de arquivo;
- Política de permissões e sandbox;
- Submódulos externos de skills;
- Configuração de MCPs;
- Hooks de validação.

Instalação recomendada, com inspeção prévia do script:

```bash
mkdir eventquest
cd eventquest
git init

curl -fsSL \
  https://raw.githubusercontent.com/solanabr/solana-ai-kit/main/install.sh \
  -o /tmp/install-solana-ai-kit.sh

sed -n '1,240p' /tmp/install-solana-ai-kit.sh
bash /tmp/install-solana-ai-kit.sh

git submodule update --init --recursive
claude
```

Alternativa para Codex, Cursor, Windsurf, Copilot ou outra IA que utilize `.agents/`:

```bash
bash /tmp/install-solana-ai-kit.sh --agents
```

Não instalar simultaneamente:

- O plugin do Claude Code;
- E a instalação completa pelo `install.sh`;

Isso pode carregar comandos, hooks e MCPs em duplicidade.

### 0.3 Verificação inicial obrigatória

Na primeira execução, a IA deve rodar:

```text
/doctor
/setup-mcp
```

MCPs prioritários:

1. `solana-dev` — documentação oficial da Solana;
2. Context7 — documentação das versões reais das bibliotecas;
3. Playwright — testes E2E no navegador;
4. Surfpool — validator local e testes;
5. Helius — RPC, webhooks e DAS API, quando uma chave estiver disponível.

O projeto não deve depender de Helius para o fluxo mínimo funcionar. Deve ser possível usar um RPC Devnet compatível para o MVP.

### 0.4 Agentes que devem participar

A IA principal deve delegar ou assumir explicitamente os seguintes papéis:

| Etapa | Agente do kit |
|---|---|
| Arquitetura, contas e PDAs | `solana-architect` |
| Programa Anchor | `anchor-engineer` |
| Frontend e carteiras | `solana-frontend-engineer` |
| API, QR, banco e indexador | `rust-backend-engineer` |
| Infraestrutura e CI | `devops-engineer` |
| Testes e validação | `solana-qa-engineer` |
| Documentação final | `tech-docs-writer` |

Fluxo recomendado:

```text
solana-architect
→ anchor-engineer
→ rust-backend-engineer
→ solana-frontend-engineer
→ solana-qa-engineer
→ devops-engineer
→ tech-docs-writer
```

Exemplo de instrução para a IA:

```text
Crie um agent team para implementar o EventQuest:
solana-architect para arquitetura e PDAs,
anchor-engineer para o programa,
rust-backend-engineer para API, QR e indexador,
solana-frontend-engineer para o Next.js,
solana-qa-engineer para testes locais e na Devnet,
e devops-engineer para CI e deploy.
```

### 0.5 Comandos do kit que fazem parte do processo

A IA deve utilizar, quando aplicáveis:

```text
/plan-feature
/scaffold
/build-program
/build-app
/generate-idl-client
/test-rust
/test-ts
/test-and-fix
/profile-cu
/audit-solana
/audit-infra
/product-review
/diff-review
/setup-ci-cd
/deploy devnet
/write-docs
```

Executar um comando do kit não substitui a necessidade de mostrar:

- O comando real executado;
- O resultado;
- Os testes que passaram;
- As falhas encontradas;
- Os arquivos modificados.

### 0.6 Stack obrigatória do projeto

A IA não deve escolher outra stack sem registrar uma decisão arquitetural e justificar a mudança.

#### Frontend

```text
Next.js 15+
React 19+
TypeScript
@solana/kit
Wallet Standard
TanStack Query
Zod
Tailwind CSS
Playwright
Vitest
```

#### Backend

```text
Rust
Axum 0.8+
Tokio
sqlx
PostgreSQL
Redis
SSE
OpenTelemetry
```

#### Blockchain

```text
Solana
Anchor
Rust
IDL
Client tipado gerado a partir do IDL
```

#### Testes Solana

```text
Anchor tests
LiteSVM
Mollusk
Surfpool ou solana-test-validator
Teste de fumaça real na Devnet
```

### 0.7 Rede obrigatória

Durante o desenvolvimento:

```text
local validator → Solana Devnet
```

A entrega do MVP somente será aceita quando estiver funcional na:

```text
Solana Devnet
```

Mainnet não faz parte do escopo.

Todas as configurações devem impedir deploy acidental em Mainnet.

### 0.8 Evidências obrigatórias da entrega na Devnet

A IA não pode declarar o projeto concluído apenas porque:

- Compila;
- Passa em testes com mocks;
- Funciona no validator local;
- A interface abre;
- Uma transação foi simulada.

A entrega deve conter:

```text
docs/DEVNET_TEST_REPORT.md
deployments/devnet.json
scripts/devnet-smoke-test.ts
```

O arquivo `deployments/devnet.json` deve registrar, sem segredos:

```json
{
  "cluster": "devnet",
  "programId": "PROGRAM_ID",
  "deployedAt": "ISO-8601",
  "deploySignature": "SIGNATURE",
  "idlSha256": "HASH",
  "gitCommit": "COMMIT_SHA",
  "explorerBaseUrl": "https://explorer.solana.com/?cluster=devnet"
}
```

O relatório da Devnet deve incluir:

- Program ID;
- Assinatura do deploy;
- Endereço do Event PDA criado no teste;
- Endereço do Checkpoint PDA;
- Endereço do Attendance PDA;
- Assinatura de um check-in confirmado;
- Link do Solana Explorer para cada transação;
- Resultado do teste de check-in duplicado;
- Carteiras de teste públicas;
- Data e horário do teste;
- Commit exato testado;
- Versões de Solana CLI, Anchor, Rust, Node e pnpm;
- Limitações conhecidas.

Nunca incluir chaves privadas ou seed phrases nesses arquivos.

### 0.9 Teste de fumaça obrigatório na Devnet

O script deve executar o fluxo real:

```text
1. Confirmar que o RPC está em Devnet;
2. Confirmar que o Program ID está executável;
3. Financiar carteiras de teste pela faucet, quando necessário;
4. Criar evento;
5. Criar checkpoint;
6. Gerar um desafio equivalente ao fluxo do backend;
7. Enviar check-in assinado pelo participante e pelo attestor;
8. Confirmar a transação;
9. Ler e validar a Attendance PDA;
10. Tentar repetir o mesmo check-in;
11. Confirmar que a repetição falha;
12. Produzir relatório sem segredos.
```

O script deve encerrar com código diferente de zero se qualquer validação falhar.

---

## 1. Visão geral

O **EventQuest** é uma plataforma de presença e gamificação para eventos presenciais, como:

- Workshops;
- Congressos;
- Feiras;
- Hackathons;
- Convenções;
- Treinamentos;
- Eventos corporativos;
- Visitas a estandes;
- Trilhas educacionais.

Cada participante conecta uma carteira Solana, entra em um evento e realiza check-ins em atividades por meio de **QR Codes rotativos**, atualizados automaticamente a cada intervalo configurável.

Quando um QR Code válido é escaneado, o sistema:

1. Confirma que o código foi emitido pelo EventQuest;
2. Verifica se ele ainda está dentro do prazo;
3. Vincula a solicitação à carteira autenticada;
4. Impede presença duplicada na mesma atividade;
5. Prepara uma transação Solana;
6. Exige a assinatura da carteira do participante;
7. Registra a presença on-chain;
8. Atualiza pontos, missões e progresso no frontend.

O QR Code rotativo reduz fraudes por fotografia, captura de tela ou compartilhamento posterior. Ele não elimina completamente o compartilhamento em tempo real, mas reduz significativamente sua janela de utilização.

---

## 2. Objetivo do produto

Criar uma aplicação que permita aos organizadores:

- Criar e administrar eventos;
- Cadastrar atividades e pontos de presença;
- Exibir QR Codes rotativos em telas;
- Comprovar presença de participantes;
- Criar missões e recompensas;
- Acompanhar métricas em tempo real;
- Emitir certificados ou badges verificáveis;
- Manter um histórico auditável na Solana.

Para os participantes, o sistema deve permitir:

- Conectar uma carteira Solana;
- Autenticar-se assinando uma mensagem;
- Entrar em um evento;
- Escanear QR Codes;
- Assinar a transação de check-in;
- Acompanhar pontos e missões;
- Consultar o histórico on-chain;
- Receber um passaporte ou badge digital após concluir o evento.

---

## 3. Princípio de arquitetura

Utilizar uma arquitetura híbrida.

### Dados on-chain

Devem ficar na Solana somente os dados que precisam ser:

- Imutáveis;
- Auditáveis;
- Verificáveis publicamente;
- Vinculados à carteira;
- Independentes do banco de dados da plataforma.

Exemplos:

- Identificador do evento;
- Identificador do checkpoint;
- Carteira do participante;
- Data aproximada do check-in;
- Hash do desafio de presença;
- Pontos concedidos;
- Quantidade de checkpoints concluídos;
- Status de conclusão do evento.

### Dados off-chain

Devem ficar no backend e PostgreSQL:

- Nome e descrição do evento;
- Imagens;
- Endereço físico;
- Dados pessoais;
- E-mail;
- Nome do participante;
- Configurações detalhadas;
- Logs técnicos;
- Informações de dispositivos;
- Métricas;
- Missões complexas;
- Relatórios;
- Dados que possam precisar ser alterados ou apagados;
- Conteúdo protegido pela LGPD.

### Regra de privacidade

Não registrar na blockchain:

- Nome;
- CPF;
- E-mail;
- Telefone;
- Endereço;
- IP;
- Geolocalização precisa;
- Identificadores de dispositivo;
- Documentos pessoais.

A carteira pública será o identificador on-chain do participante.

---

## 4. Escopo recomendado do MVP

O MVP deve conter:

1. Autenticação por carteira Solana;
2. Painel administrativo;
3. Criação de evento;
4. Cadastro de checkpoints;
5. Tela pública de QR Code rotativo;
6. Leitor de QR Code no celular;
7. Validação server-side do QR;
8. Registro de presença on-chain;
9. Prevenção de presença duplicada;
10. Pontuação por checkpoint;
11. Tela de progresso do participante;
12. Dashboard básico do organizador;
13. Sincronização entre eventos on-chain e PostgreSQL;
14. Execução em Solana Devnet;
15. Testes automatizados dos fluxos críticos.

### Fora do MVP

Não implementar na primeira versão:

- Marketplace;
- Token próprio;
- Pagamentos;
- Ranking global entre eventos;
- Arbitragem;
- Votação;
- Integração com catracas;
- Reconhecimento facial;
- Geofencing obrigatório;
- Aplicativo nativo;
- NFT negociável;
- Recompensas financeiras;
- Suporte multi-chain.

Esses recursos podem ser incluídos depois.

---

## 5. Papéis do sistema

### 5.1 Administrador da plataforma

Pode:

- Gerenciar organizações;
- Suspender eventos;
- Consultar logs;
- Gerenciar configurações globais;
- Auditar transações;
- Reprocessar sincronizações.

### 5.2 Organizador

Pode:

- Criar um evento;
- Editar os dados do evento;
- Adicionar outros administradores;
- Cadastrar checkpoints;
- Configurar pontos;
- Ativar e desativar check-ins;
- Abrir a tela de exibição do QR;
- Consultar participantes;
- Exportar relatórios;
- Finalizar o evento.

### 5.3 Operador de checkpoint

Pode:

- Abrir a tela de QR Code de um checkpoint específico;
- Ativar ou pausar o checkpoint;
- Consultar a quantidade de check-ins;
- Validar problemas presencialmente.

Não pode editar o evento inteiro, salvo quando também for organizador.

### 5.4 Participante

Pode:

- Conectar a carteira;
- Entrar em eventos públicos ou privados;
- Escanear QR Codes;
- Assinar check-ins;
- Consultar pontos;
- Consultar missões;
- Consultar transações;
- Receber badge ou passaporte.

---

## 6. Conceitos principais

### Evento

Representa o congresso, workshop, feira ou encontro.

### Checkpoint

Representa um local ou atividade que permite comprovar presença.

Exemplos:

- Entrada principal;
- Palestra de abertura;
- Workshop de Solana;
- Estande de patrocinador;
- Sala de networking;
- Encerramento.

### Check-in

Registro de que uma carteira esteve em determinado checkpoint.

### Missão

Regra composta por um ou mais checkpoints.

Exemplos:

- Visitar três estandes;
- Participar de duas palestras;
- Realizar o check-in de entrada e de encerramento;
- Completar todos os workshops de uma trilha.

### Pontos

Quantidade concedida quando o participante conclui um checkpoint.

### Passaporte

Representação do histórico ou conclusão do participante. No MVP, pode ser uma tela da aplicação baseada nos registros on-chain. Em uma fase posterior, pode ser emitido como cNFT.

---

## 7. Jornada principal do participante

1. O participante abre o EventQuest no celular;
2. Conecta Phantom, Solflare, Backpack ou outra carteira compatível com Wallet Standard;
3. Assina uma mensagem de autenticação;
4. Seleciona ou acessa o evento por link;
5. Abre o leitor de QR Code;
6. Escaneia o QR exibido no local;
7. O frontend envia o token do QR ao backend;
8. O backend valida o token;
9. O backend confirma que a carteira ainda não fez aquele check-in;
10. O backend prepara e assina parcialmente uma transação;
11. O frontend solicita a assinatura da carteira;
12. A transação é enviada à Solana;
13. O programa registra a presença;
14. O indexador confirma a transação;
15. A interface exibe sucesso, pontos e progresso atualizado.

---

# 8. Prova de presença com QR Code rotativo

## 8.1 Objetivo

Um QR Code estático poderia ser fotografado e compartilhado durante ou depois do evento. Para reduzir esse problema, cada checkpoint deve mostrar um QR Code que muda automaticamente.

Configuração inicial recomendada:

```env
QR_ROTATION_SECONDS=15
QR_GRACE_SECONDS=5
CHECKIN_GRANT_SECONDS=30
```

Significado:

- O QR principal muda a cada 15 segundos;
- O backend aceita até 5 segundos adicionais para compensar latência;
- Depois da leitura, o participante recebe uma autorização individual válida por 30 segundos.

O intervalo deve ser configurável por evento ou checkpoint, respeitando limites seguros:

```text
mínimo: 10 segundos
padrão: 15 segundos
máximo recomendado: 60 segundos
```

Não permitir intervalos menores que 10 segundos no MVP, pois podem prejudicar:

- Leitura pela câmera;
- Usuários com celulares lentos;
- Conexões ruins;
- Acessibilidade;
- Tempo de assinatura da carteira.

---

## 8.2 Arquitetura do QR

O QR não deve conter uma transação pronta e não deve conter segredos.

Ele deve conter uma URL HTTPS:

```text
https://app.eventquest.example/check-in?token=<TOKEN_ASSINADO>
```

O token deve ser criado exclusivamente pelo backend.

Formato recomendado: JWS/JWT compacto assinado pelo servidor.

Payload conceitual:

```json
{
  "v": 1,
  "scope": "eventquest:attendance",
  "eventId": "uuid-do-evento",
  "checkpointId": "uuid-do-checkpoint",
  "jti": "nonce-aleatorio-de-128-bits",
  "iat": 1784851200,
  "exp": 1784851215
}
```

Campos:

| Campo | Descrição |
|---|---|
| `v` | Versão do formato |
| `scope` | Impede reutilização em outro contexto |
| `eventId` | Evento relacionado |
| `checkpointId` | Checkpoint relacionado |
| `jti` | Identificador aleatório da janela |
| `iat` | Momento de emissão |
| `exp` | Momento de expiração |

### Requisitos do nonce

O `jti` deve:

- Ser imprevisível;
- Ter no mínimo 128 bits de entropia;
- Ser gerado com CSPRNG;
- Não ser incremental;
- Não ser baseado apenas no timestamp;
- Ser armazenado temporariamente no Redis.

Exemplo de chave:

```text
eventquest:qr:{checkpointId}:{jti}
```

TTL:

```text
QR_ROTATION_SECONDS + QR_GRACE_SECONDS + 10
```

---

## 8.3 Assinatura do token

Opções aceitáveis:

### Opção A — HMAC SHA-256

Mais simples para o MVP.

```text
JWT com HS256
```

O segredo deve existir apenas no backend.

### Opção B — Ed25519

Mais adequada caso outros serviços precisem verificar o QR sem acesso ao segredo.

A chave privada deve ficar no backend ou em KMS/HSM. A chave pública pode ser distribuída.

### Decisão recomendada para o MVP

Usar `HS256` para o QR off-chain e uma chave Solana separada para assinar parcialmente a transação on-chain.

Não reutilizar:

- Chave da carteira do organizador;
- Chave de deploy do programa;
- Chave de mint;
- Chave do servidor de produção;
- Segredo JWT da autenticação.

Cada finalidade deve ter seu próprio segredo ou keypair.

---

## 8.4 Atualização visual do QR

A página de exibição do checkpoint deve obter o QR atual do backend.

Solução recomendada:

- Server-Sent Events — SSE;
- Fallback por polling.

Endpoint SSE:

```http
GET /api/v1/public/checkpoints/{checkpointId}/qr-stream
```

Evento recebido:

```json
{
  "token": "eyJ...",
  "qrUrl": "https://app.eventquest.example/check-in?token=eyJ...",
  "issuedAt": "2026-07-24T22:00:00-03:00",
  "expiresAt": "2026-07-24T22:00:15-03:00",
  "remainingSeconds": 15
}
```

O frontend deve:

- Renderizar o QR;
- Mostrar o nome do checkpoint;
- Mostrar um contador regressivo;
- Trocar o código sem recarregar a página;
- Exibir estado de conexão;
- Ocultar o token em logs;
- Nunca armazenar o token em `localStorage`.

Fallback:

```http
GET /api/v1/public/checkpoints/{checkpointId}/current-qr
```

Polling recomendado:

```text
a cada 2 segundos
```

O backend deve devolver o mesmo token durante a mesma janela, evitando gerar um novo token em cada request.

---

## 8.5 Algoritmo de rotação

Pseudocódigo:

```ts
function getRotationWindow(nowMs: number, rotationSeconds: number): number {
  return Math.floor(nowMs / (rotationSeconds * 1000));
}

async function getOrCreateCurrentQr(checkpointId: string) {
  const checkpoint = await loadActiveCheckpoint(checkpointId);
  const window = getRotationWindow(Date.now(), checkpoint.rotationSeconds);

  const cacheKey = `eventquest:qr-window:${checkpointId}:${window}`;

  const cached = await redis.get(cacheKey);
  if (cached) return JSON.parse(cached);

  const issuedAt = window * checkpoint.rotationSeconds;
  const expiresAt = issuedAt + checkpoint.rotationSeconds;
  const jti = secureRandom128Bits();

  const payload = {
    v: 1,
    scope: "eventquest:attendance",
    eventId: checkpoint.eventId,
    checkpointId,
    jti,
    iat: issuedAt,
    exp: expiresAt
  };

  const token = signQrToken(payload);

  const result = {
    token,
    qrUrl: `${PUBLIC_APP_URL}/check-in?token=${encodeURIComponent(token)}`,
    issuedAt,
    expiresAt
  };

  await redis.set(cacheKey, JSON.stringify(result), {
    EX: checkpoint.rotationSeconds + QR_GRACE_SECONDS + 10,
    NX: true
  });

  await redis.set(
    `eventquest:qr:${checkpointId}:${jti}`,
    JSON.stringify(payload),
    { EX: checkpoint.rotationSeconds + QR_GRACE_SECONDS + 10 }
  );

  return result;
}
```

Em caso de concorrência, usar:

- `SET NX`;
- Lock distribuído curto;
- Ou script Lua no Redis.

Todos os clientes exibindo o mesmo checkpoint devem receber o mesmo QR durante a mesma janela.

---

# 9. Fluxo seguro de check-in

## 9.1 Autenticação da carteira

Conectar uma carteira não comprova, sozinho, que o usuário controla a chave.

O backend deve fornecer um nonce de autenticação:

```http
POST /api/v1/auth/nonce
```

Resposta:

```json
{
  "nonce": "nonce-aleatorio",
  "message": "eventquest.example quer autenticar esta carteira Solana...",
  "expiresAt": "2026-07-24T22:05:00-03:00"
}
```

O participante assina a mensagem.

Depois:

```http
POST /api/v1/auth/verify
```

Body:

```json
{
  "wallet": "CARTEIRA_BASE58",
  "message": "MENSAGEM_EXATA",
  "signature": "ASSINATURA_BASE58"
}
```

O backend:

1. Confirma o nonce;
2. Confirma domínio, data e expiração;
3. Verifica a assinatura Ed25519;
4. Marca o nonce como utilizado;
5. Cria uma sessão;
6. Emite cookie `HttpOnly`, `Secure` e `SameSite=Lax`.

Preferir Sign-In With Solana quando a carteira suportar. Manter fallback por assinatura de mensagem.

---

## 9.2 Validação do QR escaneado

Endpoint:

```http
POST /api/v1/check-ins/validate
Authorization: sessão autenticada
Idempotency-Key: UUID
```

Body:

```json
{
  "qrToken": "eyJ...",
  "wallet": "CARTEIRA_BASE58"
}
```

Validações obrigatórias:

1. Sessão válida;
2. Carteira da sessão igual à carteira informada;
3. Assinatura do QR válida;
4. `scope` correto;
5. Versão suportada;
6. Evento existente;
7. Checkpoint existente;
8. Checkpoint pertencente ao evento;
9. Evento ativo;
10. Checkpoint ativo;
11. Horário dentro da janela;
12. Token encontrado no Redis;
13. QR não revogado;
14. Carteira inscrita, quando o evento exigir inscrição;
15. Ausência de check-in anterior;
16. Ausência de transação pendente equivalente;
17. Rate limit não excedido.

Depois da validação, o backend cria uma autorização individual:

```json
{
  "grantId": "uuid",
  "eventId": "uuid-do-evento",
  "checkpointId": "uuid-do-checkpoint",
  "wallet": "CARTEIRA_BASE58",
  "challengeHash": "SHA256_DO_TOKEN",
  "expiresAt": "2026-07-24T22:00:45-03:00"
}
```

Redis:

```text
eventquest:checkin-grant:{grantId}
```

TTL:

```text
CHECKIN_GRANT_SECONDS
```

A autorização deve ser vinculada a:

- Carteira;
- Evento;
- Checkpoint;
- Hash do QR;
- Sessão;
- Idempotency key.

Ela não pode ser usada por outra carteira.

---

## 9.3 Preparação da transação

Endpoint:

```http
POST /api/v1/check-ins/{grantId}/prepare-transaction
```

O backend:

1. Carrega o grant;
2. Confirma que ainda não expirou;
3. Confirma que não está concluído;
4. Confirma novamente que não existe presença on-chain;
5. Obtém um blockhash recente;
6. Monta a instrução `check_in`;
7. Define o participante como fee payer no MVP;
8. Assina parcialmente como `checkpoint_attestor`;
9. Salva o status como `TRANSACTION_PREPARED`;
10. Retorna a transação serializada em Base64.

Resposta:

```json
{
  "transaction": "BASE64",
  "network": "devnet",
  "expiresAt": "2026-07-24T22:01:15-03:00",
  "grantId": "uuid"
}
```

A transação deve exigir duas assinaturas:

- Participante;
- Attestor autorizado pelo checkpoint.

### Por que usar o attestor como coassinante

O programa on-chain não precisa interpretar JWT ou acessar Redis.

O backend só coassina depois de validar:

- QR;
- Tempo;
- Carteira;
- Evento;
- Duplicidade;
- Regras do checkpoint.

O programa verifica que o attestor esperado também assinou a transação.

---

## 9.4 Assinatura e envio pelo participante

O frontend:

1. Desserializa a transação;
2. Verifica que a rede é a esperada;
3. Exibe uma mensagem clara;
4. Solicita assinatura;
5. Envia a transação;
6. Obtém a assinatura da transação;
7. Exibe status pendente;
8. Monitora a confirmação.

Nunca assinar silenciosamente.

Nunca solicitar assinatura de transação com:

- Transferência de SOL;
- Transferência de tokens;
- Aprovação de delegate;
- Mudança de autoridade;
- Instruções não documentadas.

A transação de check-in deve conter somente as instruções esperadas para a presença e, quando necessário, Compute Budget.

---

## 9.5 Confirmação

Após o envio:

```http
POST /api/v1/check-ins/{grantId}/submitted
```

Body:

```json
{
  "signature": "ASSINATURA_DA_TRANSACAO"
}
```

O backend deve verificar:

- Assinatura no formato correto;
- Transação na rede esperada;
- Programa correto;
- Evento correto;
- Checkpoint correto;
- Participante correto;
- Attestor correto;
- Attendance PDA correto;
- Status confirmado.

O indexador on-chain é a fonte final da confirmação.

Estados:

```text
QR_VALIDATED
TRANSACTION_PREPARED
TRANSACTION_SUBMITTED
CONFIRMED
FAILED
EXPIRED
REJECTED
```

---

# 10. Limitações da prova por QR

O QR rotativo reduz fraude, mas não prova localização física de forma absoluta.

Ainda é possível que alguém:

- Faça uma transmissão ao vivo da tela;
- Envie uma fotografia imediatamente;
- Compartilhe o QR por chamada de vídeo;
- Utilize acesso remoto ao dispositivo.

Mitigações opcionais:

1. Intervalo curto;
2. Exigir carteira autenticada antes do scan;
3. Limitar uma carteira por checkpoint;
4. Rate limiting;
5. Inscrição prévia;
6. Checkpoint aberto apenas durante a atividade;
7. Rede Wi-Fi local;
8. Geofencing aproximado com consentimento;
9. Bluetooth beacon;
10. Código adicional informado pelo palestrante;
11. Validação visual por operador;
12. Múltiplos checkpoints em momentos diferentes.

Não afirmar que o QR sozinho fornece prova criptográfica absoluta de presença física.

O sistema fornece:

> Uma evidência forte de que determinada carteira teve acesso a um desafio temporário exibido no checkpoint e assinou uma transação dentro da janela permitida.

---

# 11. Programa on-chain

## 11.1 Tecnologia

Sugestão:

- Rust;
- Anchor;
- Solana Devnet no desenvolvimento;
- Solana Mainnet apenas após auditoria e testes.

Nome conceitual:

```text
eventquest_program
```

---

## 11.2 Contas PDA

### EventAccount

Seeds:

```text
["event", organizer_pubkey, event_external_id_hash]
```

Campos conceituais:

```rust
pub struct EventAccount {
    pub authority: Pubkey,
    pub external_id_hash: [u8; 32],
    pub starts_at: i64,
    pub ends_at: i64,
    pub status: EventStatus,
    pub checkpoint_count: u32,
    pub total_checkins: u64,
    pub bump: u8,
}
```

### CheckpointAccount

Seeds:

```text
["checkpoint", event_pda, checkpoint_external_id_hash]
```

Campos:

```rust
pub struct CheckpointAccount {
    pub event: Pubkey,
    pub attestor: Pubkey,
    pub external_id_hash: [u8; 32],
    pub opens_at: i64,
    pub closes_at: i64,
    pub points: u32,
    pub active: bool,
    pub total_checkins: u64,
    pub bump: u8,
}
```

### ParticipantEventAccount

Seeds:

```text
["participant", event_pda, participant_pubkey]
```

Campos:

```rust
pub struct ParticipantEventAccount {
    pub event: Pubkey,
    pub participant: Pubkey,
    pub points: u64,
    pub checkin_count: u32,
    pub completed: bool,
    pub created_at: i64,
    pub updated_at: i64,
    pub bump: u8,
}
```

### AttendanceAccount

Seeds:

```text
[
  "attendance",
  event_pda,
  checkpoint_pda,
  participant_pubkey
]
```

Campos:

```rust
pub struct AttendanceAccount {
    pub event: Pubkey,
    pub checkpoint: Pubkey,
    pub participant: Pubkey,
    pub attestor: Pubkey,
    pub challenge_hash: [u8; 32],
    pub checked_in_at: i64,
    pub points_awarded: u32,
    pub bump: u8,
}
```

A combinação das seeds impede dois registros para:

```text
mesmo evento + mesmo checkpoint + mesma carteira
```

---

## 11.3 Instruções do programa

### initialize_event

Cria o evento on-chain.

Requisitos:

- `authority` deve assinar;
- Período válido;
- ID externo representado por hash de 32 bytes.

### update_event_status

Permite:

- `DRAFT`;
- `ACTIVE`;
- `PAUSED`;
- `FINISHED`;
- `CANCELLED`.

Somente a authority pode alterar.

### create_checkpoint

Cria um checkpoint.

Requisitos:

- Evento existente;
- Authority do evento assina;
- Attestor válido;
- Pontos dentro do limite;
- Horário contido ou compatível com o evento.

### update_checkpoint

Permite:

- Alterar attestor;
- Abrir ou fechar;
- Ativar ou pausar;
- Alterar pontos antes dos check-ins.

Depois que houver check-in, não alterar retroativamente os pontos já concedidos.

### check_in

Assinantes obrigatórios:

- Participante;
- Attestor do checkpoint.

Regras:

1. Evento ativo;
2. Checkpoint ativo;
3. Timestamp dentro do intervalo;
4. Attestor igual ao configurado;
5. Participante assinou;
6. Attendance PDA ainda não existe;
7. `challenge_hash` com 32 bytes;
8. Pontos copiados do checkpoint;
9. Incrementar contadores com `checked_add`;
10. Emitir evento on-chain.

### finish_participant_event

Opcional no MVP.

Marca como concluído quando os critérios forem atendidos pelo backend ou por uma configuração simplificada on-chain.

---

## 11.4 Evento emitido pelo programa

```rust
#[event]
pub struct AttendanceRecorded {
    pub event: Pubkey,
    pub checkpoint: Pubkey,
    pub participant: Pubkey,
    pub attestor: Pubkey,
    pub checked_in_at: i64,
    pub points_awarded: u32,
    pub challenge_hash: [u8; 32],
}
```

O backend deve indexar esse evento.

---

## 11.5 Regras de segurança do programa

A IA deve implementar e testar:

- Validação de signer;
- Validação de owner das contas;
- Seeds e bumps;
- `has_one`;
- Endereços esperados;
- Controle de authority;
- Checagem de overflow;
- Datas inválidas;
- Checkpoint de outro evento;
- Attestor falso;
- Conta já existente;
- Substituição de participante;
- Reutilização de Attendance PDA;
- Eventos pausados;
- Checkpoints fechados;
- Contas arbitrárias passadas pelo cliente.

Não confiar em valores de pontos enviados pelo frontend.

O programa deve ler os pontos do `CheckpointAccount`.

---

# 12. Arquitetura do backend

## 12.1 Stack definida

O backend deve utilizar a stack alinhada ao `solana-ai-kit`:

```text
Rust
Axum 0.8+
Tokio
sqlx
PostgreSQL
Redis
SSE
OpenTelemetry
tracing
serde
validator
```

Não criar uma segunda API em Node.js, Next.js API Routes ou Java para o MVP.

O Next.js pode utilizar Server Components e Route Handlers somente para funções específicas da camada web, como:

- Proxy seguro;
- Leitura de sessão;
- Renderização;
- Metadados;
- Recursos que não duplicam a API principal.

As regras de negócio, QR rotativo, grants, construção de transações e indexação devem ficar na API Rust.

### Organização recomendada

```text
apps/api
apps/indexer
```

O `apps/api` deve atender HTTP/SSE.

O `apps/indexer` deve ser um processo separado, podendo reutilizar crates e módulos compartilhados.

### Persistência

Usar migrations do `sqlx`.

A aplicação deve iniciar somente após confirmar que:

- PostgreSQL está disponível;
- Migrations são compatíveis;
- Redis está disponível;
- RPC está configurado para a rede permitida.

### Cliente Solana no backend

Usar bibliotecas atuais e compatíveis com as versões fixadas do projeto.

A IA deve consultar documentação oficial e Context7 antes de definir dependências. Não inventar métodos ou misturar versões incompatíveis.

O backend deve:

- Construir transações;
- Validar contas;
- Coassinar como attestor;
- Consultar PDAs;
- Verificar confirmações;
- Indexar eventos do programa.

O keypair do attestor deve ficar fora do repositório.

### Regra de rede

A API deve falhar na inicialização quando:

```text
SOLANA_NETWORK != devnet
```

para o ambiente de demonstração solicitado.

Uma liberação futura para Mainnet exigirá configuração e revisão separadas.

## 12.2 Responsabilidades do backend

- Autenticar carteiras;
- Administrar sessões;
- Criar eventos;
- Criar checkpoints;
- Gerar QR Codes;
- Validar tokens;
- Gerar grants;
- Construir transações;
- Assinar como attestor;
- Enviar ou acompanhar transações;
- Indexar eventos on-chain;
- Sincronizar PostgreSQL;
- Enviar atualizações em tempo real;
- Gerar relatórios;
- Aplicar rate limit;
- Produzir logs de auditoria.

---

## 12.3 Endpoints principais

### Autenticação

```http
POST /api/v1/auth/nonce
POST /api/v1/auth/verify
POST /api/v1/auth/logout
GET  /api/v1/auth/me
```

### Eventos

```http
POST   /api/v1/events
GET    /api/v1/events
GET    /api/v1/events/{eventId}
PATCH  /api/v1/events/{eventId}
POST   /api/v1/events/{eventId}/publish
POST   /api/v1/events/{eventId}/pause
POST   /api/v1/events/{eventId}/finish
```

### Checkpoints

```http
POST   /api/v1/events/{eventId}/checkpoints
GET    /api/v1/events/{eventId}/checkpoints
GET    /api/v1/checkpoints/{checkpointId}
PATCH  /api/v1/checkpoints/{checkpointId}
POST   /api/v1/checkpoints/{checkpointId}/activate
POST   /api/v1/checkpoints/{checkpointId}/pause
```

### QR público

```http
GET /api/v1/public/checkpoints/{checkpointId}/current-qr
GET /api/v1/public/checkpoints/{checkpointId}/qr-stream
```

A rota de QR deve exigir um `displayAccessToken` de curta ou média duração, para evitar que qualquer pessoa descubra checkpoints privados.

### Check-in

```http
POST /api/v1/check-ins/validate
POST /api/v1/check-ins/{grantId}/prepare-transaction
POST /api/v1/check-ins/{grantId}/submitted
GET  /api/v1/check-ins/{grantId}
```

### Participante

```http
GET /api/v1/me/events
GET /api/v1/me/events/{eventId}/progress
GET /api/v1/me/events/{eventId}/attendances
GET /api/v1/me/transactions
```

### Dashboard

```http
GET /api/v1/events/{eventId}/dashboard
GET /api/v1/events/{eventId}/participants
GET /api/v1/events/{eventId}/check-ins
GET /api/v1/events/{eventId}/export.csv
```

---

# 13. Modelo de dados PostgreSQL

## organizations

```text
id UUID PK
name VARCHAR
slug VARCHAR UNIQUE
created_at TIMESTAMPTZ
updated_at TIMESTAMPTZ
```

## organization_members

```text
organization_id UUID FK
wallet_address VARCHAR
role ENUM
created_at TIMESTAMPTZ
PRIMARY KEY (organization_id, wallet_address)
```

## events

```text
id UUID PK
organization_id UUID FK
name VARCHAR
slug VARCHAR
description TEXT
banner_url TEXT
location_name VARCHAR
location_address TEXT
timezone VARCHAR
starts_at TIMESTAMPTZ
ends_at TIMESTAMPTZ
status ENUM
visibility ENUM
solana_network ENUM
onchain_event_address VARCHAR NULL
onchain_create_signature VARCHAR NULL
created_by_wallet VARCHAR
created_at TIMESTAMPTZ
updated_at TIMESTAMPTZ
```

## checkpoints

```text
id UUID PK
event_id UUID FK
name VARCHAR
description TEXT
points INTEGER
rotation_seconds INTEGER
opens_at TIMESTAMPTZ
closes_at TIMESTAMPTZ
status ENUM
attestor_pubkey VARCHAR
onchain_checkpoint_address VARCHAR NULL
display_token_hash VARCHAR NULL
created_at TIMESTAMPTZ
updated_at TIMESTAMPTZ
```

## participants

```text
id UUID PK
wallet_address VARCHAR UNIQUE
display_name VARCHAR NULL
email VARCHAR NULL
created_at TIMESTAMPTZ
updated_at TIMESTAMPTZ
```

## event_participants

```text
event_id UUID FK
participant_id UUID FK
status ENUM
points BIGINT
checkin_count INTEGER
completed BOOLEAN
joined_at TIMESTAMPTZ
completed_at TIMESTAMPTZ NULL
PRIMARY KEY (event_id, participant_id)
```

## checkin_attempts

```text
id UUID PK
grant_id UUID UNIQUE
event_id UUID FK
checkpoint_id UUID FK
participant_id UUID FK
qr_jti_hash VARCHAR
challenge_hash VARCHAR
idempotency_key VARCHAR
status ENUM
transaction_signature VARCHAR NULL
failure_code VARCHAR NULL
failure_message TEXT NULL
expires_at TIMESTAMPTZ
created_at TIMESTAMPTZ
updated_at TIMESTAMPTZ
UNIQUE (checkpoint_id, participant_id)
```

## attendances

```text
id UUID PK
event_id UUID FK
checkpoint_id UUID FK
participant_id UUID FK
onchain_attendance_address VARCHAR UNIQUE
transaction_signature VARCHAR UNIQUE
block_time TIMESTAMPTZ
slot BIGINT
points_awarded INTEGER
challenge_hash VARCHAR
confirmed_at TIMESTAMPTZ
created_at TIMESTAMPTZ
UNIQUE (checkpoint_id, participant_id)
```

## chain_sync_cursors

```text
network VARCHAR
program_id VARCHAR
last_processed_slot BIGINT
updated_at TIMESTAMPTZ
PRIMARY KEY (network, program_id)
```

## audit_logs

```text
id UUID PK
actor_wallet VARCHAR NULL
action VARCHAR
resource_type VARCHAR
resource_id UUID NULL
metadata JSONB
ip_hash VARCHAR NULL
created_at TIMESTAMPTZ
```

Não salvar o IP em texto puro. Caso necessário para antifraude, armazenar hash rotacionável e aplicar política de retenção.

---

# 14. Redis

Chaves sugeridas:

```text
eventquest:auth-nonce:{nonce}
eventquest:session:{sessionId}
eventquest:qr-window:{checkpointId}:{window}
eventquest:qr:{checkpointId}:{jti}
eventquest:checkin-grant:{grantId}
eventquest:rate:wallet:{wallet}:{minute}
eventquest:rate:checkpoint:{checkpointId}:{minute}
eventquest:tx-lock:{checkpointId}:{wallet}
```

O Redis não deve ser tratado como fonte definitiva dos check-ins. A fonte definitiva é a Solana, refletida no PostgreSQL pelo indexador.

---

# 15. Frontend

## 15.1 Stack definida

```text
Next.js 15+
React 19+
TypeScript
@solana/kit
Wallet Standard
TanStack Query
Zod
Tailwind CSS
Vitest
Playwright
```

### Idioma obrigatório da aplicação

Toda a experiência visível ao usuário deve estar em **inglês**.

Isso inclui:

- Navegação;
- Títulos;
- Botões;
- Formulários;
- Labels;
- Placeholders;
- Tooltips;
- Modais;
- Toasts;
- Estados vazios;
- Mensagens de carregamento;
- Mensagens de erro;
- Mensagens de confirmação;
- Conteúdo do dashboard;
- Tela de QR Code;
- Fluxo de conexão da carteira;
- Textos relacionados à Devnet;
- E-mails e notificações gerados pelo sistema;
- Metadados públicos da aplicação;
- Textos de acessibilidade, como `aria-label` e `alt`.

Idioma e locale padrão:

```text
Language: English
Locale: en-US
Timezone display: use the event timezone
Date format: locale-aware English
Time format: locale-aware English
```

Exemplos corretos:

```text
Connect wallet
Create event
Create checkpoint
Scan QR code
Confirm attendance
Waiting for wallet signature
Transaction confirmed
Already checked in
This QR code has expired
Solana Devnet
```

Não utilizar textos em português na interface final.

O código-fonte deve utilizar nomes de:

- Variáveis;
- Funções;
- Componentes;
- Rotas internas;
- Tipos;
- Schemas;
- Testes;

em inglês.

O MVP não precisa oferecer seletor de idioma. Porém, os textos não devem ficar espalhados diretamente pelos componentes. Centralizar a copy em uma estrutura preparada para futura internacionalização:

```text
apps/web/src/i18n/en-US.ts
```

ou solução equivalente.

Exemplo:

```ts
export const messages = {
  common: {
    connectWallet: "Connect wallet",
    cancel: "Cancel",
    confirm: "Confirm",
  },
  checkIn: {
    scanQrCode: "Scan QR code",
    qrExpired: "This QR code has expired.",
    alreadyCheckedIn: "You have already checked in at this checkpoint.",
    transactionConfirmed: "Attendance confirmed on Solana Devnet.",
  },
} as const;
```

Não usar tradução automática em runtime no MVP.

### Tema visual obrigatório

A aplicação deve utilizar **dark mode como tema padrão e único do MVP**, com **laranja como cor de destaque**.

Não implementar seletor light/dark no MVP.

Direção visual:

```text
Dark
Modern
High contrast
Minimal
Technology-oriented
Mobile-first
Orange accent
```

A interface não deve usar o roxo padrão associado frequentemente à Solana como cor principal. O laranja deve ser a identidade visual dominante do EventQuest.

### Paleta base

Utilizar tokens semânticos, evitando cores hexadecimais diretamente nos componentes.

```css
:root {
  color-scheme: dark;

  --background: #0b0b0c;
  --surface: #141416;
  --surface-elevated: #1c1c1f;
  --surface-hover: #232327;

  --border: #2a2a2f;
  --border-strong: #3a3a40;

  --text-primary: #f5f5f5;
  --text-secondary: #a1a1aa;
  --text-disabled: #71717a;

  --accent: #f97316;
  --accent-hover: #fb923c;
  --accent-active: #ea580c;
  --accent-soft: rgba(249, 115, 22, 0.14);
  --accent-foreground: #09090b;
  --focus-ring: #fdba74;

  --success: #22c55e;
  --warning: #f59e0b;
  --error: #ef4444;
  --info: #38bdf8;
}
```

A IA pode ajustar levemente os valores após testes de contraste, mas deve manter:

- Fundo quase preto;
- Superfícies em cinza muito escuro;
- Texto principal quase branco;
- Laranja como ação, seleção e destaque;
- Estados de erro, sucesso e aviso semanticamente distintos.

### Uso do laranja

Usar o laranja para:

- Botão primário;
- Links importantes;
- Item de navegação ativo;
- Indicador de progresso;
- Contorno de foco;
- Destaque do QR Code;
- Badges selecionados;
- Indicadores de atividade;
- Elementos on-chain importantes;
- Status visual da Devnet, quando não houver conflito semântico.

Não usar grandes blocos laranja como fundo de páginas inteiras.

Não usar laranja para:

- Mensagens de erro;
- Sucesso confirmado;
- Texto longo;
- Todos os ícones;
- Toda a navegação simultaneamente.

### Componentes

#### Botão primário

```text
Background: accent
Text: accent-foreground
Hover: accent-hover
Active: accent-active
Focus: visible orange ring
Disabled: reduced contrast and no hover
```

Texto de exemplo:

```text
Connect wallet
Create event
Confirm attendance
```

#### Botão secundário

```text
Background: surface-elevated
Border: border-strong
Text: text-primary
Hover: surface-hover
```

#### Cards

```text
Background: surface
Border: border
Border radius: 12px to 16px
Shadow: subtle
```

#### Inputs

```text
Background: surface-elevated
Border: border
Text: text-primary
Placeholder: text-disabled
Focus border/ring: accent
Error border: error
```

#### QR Code

O QR deve manter contraste máximo para leitura.

Preferência:

```text
QR modules: #000000
QR background: #ffffff
Container: dark surface
Outer accent border: orange
```

Não renderizar o QR diretamente em laranja, pois isso pode reduzir a confiabilidade da leitura em determinadas câmeras e condições de iluminação.

### Tipografia

Utilizar fonte sans-serif legível.

Opções recomendadas:

```text
Inter
Geist
system-ui
```

Hierarquia:

- Título de página: forte e direto;
- Título de card: médio;
- Corpo: legível;
- Dados on-chain: fonte monoespaçada;
- Program IDs e assinaturas: truncados visualmente, com opção de copiar.

Fonte monoespaçada sugerida:

```text
Geist Mono
JetBrains Mono
ui-monospace
```

### Acessibilidade

Requisitos mínimos:

- Contraste compatível com WCAG AA;
- Foco visível em todos os controles;
- Navegação por teclado;
- Não depender apenas de cor;
- Ícones acompanhados de texto ou `aria-label`;
- Área de toque mínima de aproximadamente 44×44 px;
- Estados de carregamento anunciados;
- Mensagens de erro associadas ao campo;
- Respeitar `prefers-reduced-motion`;
- QR Code acompanhado de instrução textual;
- Contadores não devem piscar excessivamente.

### Responsividade

Prioridade:

```text
1. Smartphone
2. Tablet
3. Desktop
4. Tela fullscreen do checkpoint
```

Breakpoints e componentes devem ser construídos mobile-first.

A tela administrativa pode usar sidebar no desktop e navegação compacta no celular.

### Identidade visual mínima

O EventQuest deve possuir:

- Wordmark ou logotipo textual;
- Ícone simples;
- Favicon;
- Marca visual baseada em laranja;
- Tela de carregamento;
- Empty states;
- Ilustrações opcionais, sem comprometer o MVP.

Evitar aparência de template genérico.

### Screenshots obrigatórios na entrega

A documentação final deve incluir screenshots em inglês de:

1. Landing page;
2. Wallet connection;
3. Event dashboard;
4. Checkpoint creation;
5. Rotating QR display;
6. Mobile QR scanner;
7. Pending wallet signature;
8. Successful Devnet check-in;
9. Duplicate check-in error;
10. Participant progress page.

Os screenshots devem mostrar o tema dark com destaque laranja.

O frontend deve ser responsivo e funcionar prioritariamente em navegadores móveis.

### Integração Solana

Para código novo, utilizar `@solana/kit`.

Não adicionar `@solana/web3.js` v1 ao frontend sem necessidade comprovada.

Caso o client padrão do Anchor utilizado seja incompatível com `@solana/kit`, gerar um client tipado a partir do IDL usando o fluxo suportado pelo kit:

```text
/generate-idl-client
```

A IA deve escolher uma única estratégia de cliente e registrá-la em:

```text
docs/adr/ADR-002-solana-client.md
```

Não misturar representações de:

- Public keys;
- Transactions;
- Signers;
- RPC clients;

sem adapters centralizados e testados.

### Carteiras

Utilizar Wallet Standard.

Requisitos:

- Phantom;
- Solflare;
- Backpack;
- Carteiras compatíveis com Wallet Standard;
- Mensagem clara sobre Devnet;
- Detecção de rede;
- Impedir envio se a configuração não for Devnet;
- Assinatura explícita de mensagens e transações;
- Nunca solicitar seed phrase.

### Experiência de Devnet

Exibir permanentemente no ambiente de demonstração:

```text
Rede: Solana Devnet
```

A interface deve mostrar:

- Program ID;
- Assinatura da transação;
- Link para o Explorer;
- Status `processed`, `confirmed` e `finalized`, quando aplicável;
- Mensagem clara quando a carteira não possui SOL de Devnet.

## 15.2 Páginas públicas

```text
/
 /events
 /events/[slug]
 /check-in
 /check-in/result
 /participant/passport
```

## 15.3 Páginas administrativas

```text
/admin
/admin/events
/admin/events/new
/admin/events/[eventId]
/admin/events/[eventId]/checkpoints
/admin/checkpoints/[checkpointId]/display
/admin/events/[eventId]/participants
/admin/events/[eventId]/analytics
```

---

## 15.4 Tela de QR do checkpoint

Deve mostrar:

- Logo do evento;
- Nome do evento;
- Nome do checkpoint;
- QR Code grande;
- Contador regressivo;
- Status ativo/pausado;
- Quantidade de check-ins confirmados;
- Indicador de conexão;
- Botão de tela cheia;
- Instrução curta para o participante.

Exemplo:

```text
Solana Workshop
Scan to confirm your attendance

New code in 12 seconds
```

Quando o checkpoint estiver pausado:

```text
Check-in temporarily unavailable
```

Não mostrar QR vencido durante perda de conexão. Substituí-lo por estado indisponível.

---

## 15.5 Leitor de QR

Requisitos:

- Solicitar permissão de câmera apenas após ação do usuário;
- Permitir câmera traseira;
- Mostrar moldura de leitura;
- Vibrar após leitura válida;
- Bloquear leituras repetidas enquanto processa;
- Aceitar colar link como fallback;
- Tratar câmera indisponível;
- Tratar QR expirado;
- Tratar carteira desconectada;
- Tratar recusa da assinatura;
- Mostrar link da transação no explorer.

Estados:

```text
IDLE
REQUESTING_CAMERA
SCANNING
VALIDATING_QR
PREPARING_TRANSACTION
AWAITING_WALLET_SIGNATURE
SUBMITTING
CONFIRMING
SUCCESS
ERROR
```

---

# 16. Indexação on-chain

O backend deve executar um worker que:

1. Acompanha o programa EventQuest;
2. Busca transações a partir de um cursor;
3. Decodifica eventos `AttendanceRecorded`;
4. Confirma programa e contas;
5. Insere ou atualiza `attendances`;
6. Atualiza pontos e contadores;
7. Publica atualização em tempo real;
8. Avança o cursor somente após persistência;
9. Reprocessa de forma idempotente.

### Requisitos

- Não depender apenas do callback do frontend;
- Não marcar presença como definitiva antes da confirmação;
- Suportar reinício do worker;
- Não duplicar registros;
- Realizar reconciliação periódica;
- Registrar slot e assinatura;
- Tratar reorganizações e níveis de commitment.

Commitment sugerido:

- UI rápida: `confirmed`;
- Fechamento e relatórios finais: reconciliar em `finalized`.

---

# 17. Passaporte e cNFT

## 17.1 MVP

No MVP, o passaporte é uma representação na aplicação composta por:

- Carteira;
- Eventos participados;
- Checkpoints concluídos;
- Pontos;
- Transações;
- Badges internos.

Os dados devem ser derivados do estado on-chain e do banco.

## 17.2 Fase 2 — Bubblegum V2

Após completar o evento, o sistema pode emitir um compressed NFT.

Exemplo:

```text
Nome: EventQuest Passport — Solana Workshop 2026
Símbolo: EQPASS
Owner: carteira do participante
Collection: coleção oficial do evento
```

Metadados externos:

```json
{
  "name": "EventQuest Passport — Solana Workshop 2026",
  "symbol": "EQPASS",
  "description": "Comprovante digital de conclusão do evento.",
  "image": "https://cdn.eventquest.example/passports/...",
  "attributes": [
    { "trait_type": "Event", "value": "Solana Workshop 2026" },
    { "trait_type": "Check-ins", "value": 8 },
    { "trait_type": "Points", "value": 120 },
    { "trait_type": "Completed", "value": true }
  ]
}
```

### Decisão recomendada

Não atualizar o cNFT a cada check-in no MVP.

Emitir o cNFT somente quando:

- O evento terminar;
- O participante cumprir os critérios;
- O estado tiver sido reconciliado.

Operações com Bubblegum V2 dependem de um RPC que suporte DAS API. Essa dependência deve ser considerada no orçamento e na infraestrutura.

---

# 18. Missões

No MVP, as missões ficam off-chain e são calculadas pelo backend com base em presenças on-chain.

Exemplo:

```json
{
  "id": "mission-uuid",
  "eventId": "event-uuid",
  "name": "Explorador",
  "type": "CHECKPOINT_COUNT",
  "requiredCount": 3,
  "rewardPoints": 20
}
```

Tipos iniciais:

```text
CHECKPOINT_COUNT
ALL_CHECKPOINTS
SPECIFIC_CHECKPOINTS
CATEGORY_COUNT
ENTRY_AND_EXIT
```

Não permitir que o frontend declare uma missão como concluída.

---

# 19. Segurança e antifraude

## 19.1 Segredos

Nunca versionar:

- Seed phrase;
- Chave privada;
- Attestor keypair;
- Authority keypair;
- JWT secret;
- QR signing secret;
- RPC API key;
- Banco de dados;
- Redis password.

Usar:

- `.env.example`;
- Secret manager;
- KMS/HSM em produção;
- Rotação de chaves;
- Chaves diferentes por ambiente.

## 19.2 Carteira do attestor

Desenvolvimento:

- Keypair local exclusivo da Devnet.

Produção:

- KMS/HSM;
- Serviço de assinatura;
- Política de acesso mínima;
- Limite de uso;
- Logs de auditoria.

## 19.3 Rate limits

Exemplos:

```text
/auth/nonce:
  10 por minuto por origem

/check-ins/validate:
  10 por minuto por carteira
  60 por minuto por checkpoint/origem

/prepare-transaction:
  5 por minuto por carteira/checkpoint

/current-qr:
  protegido por display token
```

## 19.4 Idempotência

Exigir `Idempotency-Key` nos endpoints mutáveis.

A mesma chave deve:

- Retornar a mesma resposta;
- Não criar novo grant;
- Não gerar duas transações;
- Expirar depois de período definido.

## 19.5 Logs

Não registrar:

- QR token completo;
- JWT completo;
- Cookie;
- Chave privada;
- Transação parcialmente assinada;
- Seed;
- Assinatura de autenticação em texto aberto por tempo indefinido.

Aplicar redaction.

## 19.6 Proteções web

- HTTPS obrigatório;
- CSP;
- CORS restritivo;
- CSRF quando aplicável;
- Cookies seguros;
- Validação de input;
- Limite de body;
- Proteção contra open redirect;
- Sanitização de conteúdo;
- Headers de segurança;
- Dependências fixadas e atualizadas.

---

# 20. Erros funcionais

Formato padronizado:

```json
{
  "code": "QR_EXPIRED",
  "message": "This QR code has expired. Scan the current code.",
  "requestId": "uuid"
}
```

Códigos:

```text
AUTH_REQUIRED
WALLET_MISMATCH
INVALID_SIGNATURE
QR_INVALID
QR_EXPIRED
QR_REVOKED
EVENT_NOT_ACTIVE
CHECKPOINT_NOT_ACTIVE
CHECKPOINT_NOT_OPEN
PARTICIPANT_NOT_REGISTERED
ALREADY_CHECKED_IN
CHECKIN_PENDING
GRANT_EXPIRED
TRANSACTION_EXPIRED
TRANSACTION_REJECTED
TRANSACTION_FAILED
NETWORK_MISMATCH
CHAIN_UNAVAILABLE
RATE_LIMITED
INTERNAL_ERROR
```

Mensagens devem ser claras e não expor detalhes internos.

---

# 21. Observabilidade

Implementar:

- Logs estruturados;
- Request ID;
- Métricas;
- Tracing;
- Alertas;
- Health checks.

Métricas mínimas:

```text
eventquest_qr_generated_total
eventquest_qr_validation_total
eventquest_qr_validation_failed_total
eventquest_checkin_prepared_total
eventquest_checkin_submitted_total
eventquest_checkin_confirmed_total
eventquest_checkin_failed_total
eventquest_chain_indexer_lag_slots
eventquest_chain_rpc_errors_total
eventquest_active_sse_connections
```

Health checks:

```http
GET /health/live
GET /health/ready
```

Readiness deve verificar:

- PostgreSQL;
- Redis;
- RPC Solana;
- Worker/indexador;
- Acesso ao signer, sem executar assinatura real.

---

# 22. Estrutura obrigatória do monorepo

```text
eventquest/
├── CLAUDE.md
├── CLAUDE.local.md
├── .claude/
│   ├── agents/
│   ├── commands/
│   ├── rules/
│   ├── skills/
│   └── settings.json
├── .mcp.json
├── apps/
│   ├── web/
│   │   ├── app/
│   │   ├── components/
│   │   ├── features/
│   │   ├── lib/
│   │   └── tests/
│   ├── api/
│   │   ├── src/
│   │   │   ├── auth/
│   │   │   ├── events/
│   │   │   ├── checkpoints/
│   │   │   ├── checkins/
│   │   │   ├── qr/
│   │   │   ├── blockchain/
│   │   │   └── observability/
│   │   └── migrations/
│   └── indexer/
│       └── src/
├── programs/
│   └── eventquest/
│       └── src/lib.rs
├── crates/
│   ├── eventquest-domain/
│   ├── eventquest-chain/
│   ├── eventquest-config/
│   └── eventquest-observability/
├── packages/
│   ├── chain-client/
│   ├── shared-types/
│   ├── ui/
│   └── validation/
├── scripts/
│   ├── devnet-smoke-test.ts
│   ├── verify-devnet-config.sh
│   └── generate-devnet-report.ts
├── deployments/
│   └── devnet.json
├── docs/
│   ├── DEVNET_TEST_REPORT.md
│   ├── SECURITY.md
│   ├── RUNBOOK.md
│   └── adr/
│       ├── ADR-001-architecture.md
│       ├── ADR-002-solana-client.md
│       └── ADR-003-qr-attestor.md
├── tests/
│   ├── e2e/
│   ├── integration/
│   ├── security/
│   └── devnet/
├── infrastructure/
│   ├── docker/
│   ├── github/
│   └── monitoring/
├── Anchor.toml
├── Cargo.toml
├── package.json
├── pnpm-workspace.yaml
├── docker-compose.yml
├── .env.example
└── README.md
```

### CLAUDE.md por diretório

Criar arquivos adicionais quando houver decisões específicas:

```text
apps/web/CLAUDE.md
apps/api/CLAUDE.md
apps/indexer/CLAUDE.md
programs/eventquest/CLAUDE.md
```

Esses arquivos devem conter somente contexto específico do diretório, sem duplicar integralmente a especificação.

### Versionamento do Solana AI Kit

Para trabalho individual, o kit pode permanecer ignorado pelo Git conforme a instalação padrão.

Para trabalho em equipe ou avaliação reproduzível, executar:

```text
/commit-claude-config
```

e revisar cuidadosamente o que será versionado.

Registrar no README:

- Commit ou versão do `solana-ai-kit`;
- Forma de instalação;
- MCPs opcionais;
- Comandos utilizados no projeto.

# 23. Variáveis de ambiente

```env
NODE_ENV=development
PUBLIC_APP_URL=http://localhost:3000
API_URL=http://localhost:3001

DATABASE_URL=postgresql://eventquest:eventquest@localhost:5432/eventquest
REDIS_URL=redis://localhost:6379

SOLANA_NETWORK=devnet
SOLANA_RPC_HTTP_URL=https://api.devnet.solana.com
SOLANA_RPC_WS_URL=wss://api.devnet.solana.com
SOLANA_PROGRAM_ID=

QR_SIGNING_SECRET=
AUTH_SIGNING_SECRET=
QR_ROTATION_SECONDS=15
QR_GRACE_SECONDS=5
CHECKIN_GRANT_SECONDS=30

ATTESTOR_KEYPAIR_PATH=
# Em produção, substituir por referência de KMS/HSM.

SESSION_COOKIE_NAME=eventquest_session
SESSION_TTL_SECONDS=86400

LOG_LEVEL=info
OTEL_EXPORTER_OTLP_ENDPOINT=

NEXT_PUBLIC_DEFAULT_LOCALE=en-US
NEXT_PUBLIC_THEME=dark
NEXT_PUBLIC_ACCENT_COLOR=orange
```

O `.env.example` deve conter nomes, nunca valores reais.

---

# 24. Ambiente local e Docker Compose

Serviços mínimos:

```text
postgres
redis
api
indexer
web
```

O programa deve ser testável com:

- Surfpool;
- `solana-test-validator`;
- LiteSVM;
- Mollusk.

Fluxo local esperado:

```bash
pnpm install
cargo fetch
docker compose up -d postgres redis

/doctor
/build-program
/test-rust
/test-ts
```

Quando os testes locais estiverem verdes:

```text
/deploy devnet
```

Depois do deploy:

```bash
pnpm devnet:smoke
pnpm test:e2e:devnet
```

A IA deve documentar os comandos reais gerados no projeto. Os comandos acima representam a intenção mínima, não autorizam a criação de scripts que não façam validações reais.

### Proteção contra rede errada

Criar:

```text
scripts/verify-devnet-config.sh
```

O script deve falhar quando detectar:

- RPC Mainnet;
- `Anchor.toml` apontando para Mainnet;
- Variável de ambiente Mainnet;
- Explorer sem `cluster=devnet`;
- Program ID divergente do manifesto;
- Chaves de produção.

Executar essa verificação antes de:

- Build de demonstração;
- Deploy;
- Teste E2E;
- Smoke test;
- Geração do relatório.

# 25. Estratégia de testes

Os testes devem ser organizados em quatro camadas.

## 25.1 Programa Solana — unidade e integração local

Ferramentas:

```text
cargo test
Anchor tests
LiteSVM
Mollusk
```

Testar:

1. Criação de evento;
2. Criação de checkpoint;
3. Check-in válido;
4. Check-in duplicado;
5. Participante sem assinatura;
6. Attestor sem assinatura;
7. Attestor incorreto;
8. Evento pausado;
9. Checkpoint pausado;
10. Checkpoint fora do horário;
11. Checkpoint pertencente a outro evento;
12. Pontuação adulterada;
13. Overflow;
14. PDA incorreto;
15. Challenge hash inválido;
16. Contas com owner incorreto;
17. Authority não autorizada.

Comandos do kit:

```text
/build-program
/test-rust
/profile-cu
```

## 25.2 Backend

Testar:

1. Geração de nonce;
2. Reutilização de nonce;
3. Assinatura inválida;
4. QR válido;
5. QR expirado;
6. QR com assinatura falsa;
7. QR de outro checkpoint;
8. QR revogado;
9. Grant vinculado à carteira;
10. Grant expirado;
11. Idempotência;
12. Rate limit;
13. Concorrência na rotação do QR;
14. Transação preparada com contas corretas;
15. Transação sem instruções financeiras inesperadas;
16. Confirmação on-chain;
17. Reprocessamento do indexador;
18. Retomada do cursor;
19. Falha do Redis;
20. Falha temporária do RPC.

## 25.3 Frontend

Ferramentas:

```text
Vitest
Testing Library
Playwright
```

Testar:

1. Carteira desconectada;
2. Conexão de carteira;
3. Rede diferente da Devnet;
4. Permissão de câmera negada;
5. Leitura de QR;
6. QR expirado;
7. Assinatura recusada;
8. Transação confirmada;
9. Transação falha;
10. Reconexão SSE;
11. Acessibilidade por teclado;
12. Interface responsiva;
13. Link correto para o Explorer Devnet;
14. Mensagem de saldo insuficiente;
15. Ausência de textos em português;
16. Tema dark aplicado em todas as rotas;
17. Tokens laranja aplicados a ações primárias;
18. Contraste WCAG AA;
19. Layout mobile-first;
20. QR preto sobre fundo branco dentro do container dark.

Comandos do kit:

```text
/build-app
/test-ts
```

## 25.4 E2E local

Cenário:

```text
organizador cria evento
→ cria checkpoint
→ ativa checkpoint
→ abre display
→ QR aparece
→ participante autentica
→ escaneia QR
→ assina transação
→ presença é confirmada
→ pontos aparecem
→ dashboard atualiza
→ segunda tentativa é recusada
```

Executar com Playwright e validator local.

## 25.5 Teste real na Solana Devnet

O teste de Devnet é obrigatório e não pode ser substituído pelo validator local.

Arquivo:

```text
scripts/devnet-smoke-test.ts
```

Requisitos:

- Usar RPC Devnet;
- Ler o manifesto de deploy;
- Criar contas reais na Devnet;
- Enviar transações reais;
- Aguardar `confirmed`;
- Consultar as PDAs após a confirmação;
- Testar duplicidade;
- Retornar código de erro em qualquer falha;
- Gerar evidências reproduzíveis.

Não usar mocks nesse teste.

## 25.6 Teste E2E do navegador contra Devnet

O Playwright deve executar pelo menos um cenário com:

- Frontend real;
- API real;
- PostgreSQL e Redis;
- Programa implantado na Devnet;
- Carteira de teste controlada pelo ambiente de teste;
- QR rotativo real;
- Transação real na Devnet;
- Atualização real do indexador.

É aceitável separar a assinatura da carteira em um fixture de teste seguro, desde que a transação seja real e a chave seja exclusiva da Devnet.

## 25.7 Auditorias antes da entrega

Executar:

```text
/test-and-fix
/audit-solana
/audit-infra
/product-review
/diff-review
```

A IA deve corrigir vulnerabilidades críticas e altas antes da entrega.

Descobertas não corrigidas devem ser registradas em:

```text
docs/SECURITY.md
```

com:

- Severidade;
- Evidência;
- Impacto;
- Mitigação;
- Justificativa para adiamento.

# 26. Critérios de aceite do MVP

## Interface, idioma e identidade visual

- [ ] Toda a interface visível ao usuário está em inglês;
- [ ] Não existem textos em português nas páginas, modais, erros ou toasts;
- [ ] O locale padrão é `en-US`;
- [ ] Textos estão centralizados em estrutura preparada para i18n;
- [ ] Dark mode é o único tema disponível no MVP;
- [ ] Laranja é a cor principal de destaque;
- [ ] QR Code mantém módulos pretos sobre fundo branco;
- [ ] Botões, inputs e navegação possuem foco visível;
- [ ] Contraste atende WCAG AA;
- [ ] Interface é mobile-first;
- [ ] Program IDs e assinaturas utilizam apresentação monoespaçada e opção de copiar;
- [ ] Screenshots finais demonstram o tema dark e a interface em inglês;

## Evento

- [ ] Organizador cria um evento;
- [ ] Evento recebe endereço on-chain;
- [ ] Evento pode ser ativado e pausado;
- [ ] Apenas administradores autorizados alteram o evento.

## Checkpoint

- [ ] Organizador cria checkpoint;
- [ ] Checkpoint possui período e pontos;
- [ ] Checkpoint possui attestor;
- [ ] QR muda no intervalo configurado;
- [ ] QR vencido deixa de ser aceito.

## Participante

- [ ] Carteira pode ser conectada;
- [ ] Controle da carteira é comprovado por assinatura;
- [ ] Participante consegue ler QR pelo celular;
- [ ] Transação deixa claro que se trata de check-in;
- [ ] Participante não pode realizar o mesmo check-in duas vezes.

## Blockchain

- [ ] Presença é registrada em Attendance PDA;
- [ ] Participante e attestor precisam assinar;
- [ ] Pontos vêm do CheckpointAccount;
- [ ] Evento on-chain é emitido;
- [ ] Registro pode ser consultado no explorer.

## Sincronização

- [ ] Backend indexa o check-in;
- [ ] PostgreSQL não cria duplicidade;
- [ ] Dashboard atualiza;
- [ ] Worker retoma após reinício;
- [ ] Reconciliação detecta registros ausentes.

## Devnet

- [ ] Programa implantado na Solana Devnet;
- [ ] Program ID registrado em `deployments/devnet.json`;
- [ ] Deploy verificável no Solana Explorer;
- [ ] Evento criado por transação real na Devnet;
- [ ] Checkpoint criado por transação real na Devnet;
- [ ] Check-in confirmado por transação real na Devnet;
- [ ] Attendance PDA lida e validada após a confirmação;
- [ ] Segunda tentativa do mesmo check-in falha;
- [ ] `scripts/devnet-smoke-test.ts` termina com sucesso;
- [ ] Playwright conclui o fluxo full stack contra a Devnet;
- [ ] `docs/DEVNET_TEST_REPORT.md` contém evidências e commit testado;

## Segurança

- [ ] Segredos não estão no repositório;
- [ ] QR token não é registrado integralmente;
- [ ] Rate limit está ativo;
- [ ] Nonces são single-use;
- [ ] Grants são vinculados à carteira;
- [ ] Endpoints mutáveis usam idempotência.

---

# 27. Ordem de implementação com Solana AI Kit

## Etapa 0 — Preparação do agente

1. Instalar o `solana-ai-kit`;
2. Executar `/doctor`;
3. Executar `/setup-mcp`;
4. Registrar versões;
5. Criar branch;
6. Ler esta especificação;
7. Criar ADR de arquitetura;
8. Executar `/plan-feature`.

Não iniciar implementação antes de apresentar um plano vertical.

## Etapa 1 — Scaffold

Executar:

```text
/scaffold
```

Criar:

- Monorepo;
- Next.js;
- Axum;
- Anchor;
- PostgreSQL;
- Redis;
- Testes;
- CI inicial;
- `.env.example`.

Depois:

```text
/build-program
/build-app
```

## Etapa 2 — Programa Solana

O `solana-architect` deve definir:

- PDAs;
- Authorities;
- Signers;
- Ciclo de vida;
- Eventos;
- Threat model.

O `anchor-engineer` deve implementar:

1. Evento;
2. Checkpoint;
3. Attendance;
4. Check-in;
5. Eventos on-chain;
6. Testes negativos.

Executar:

```text
/build-program
/test-rust
/profile-cu
```

## Etapa 3 — Cliente tipado

Gerar o IDL e executar:

```text
/generate-idl-client
```

Criar um pacote compartilhado:

```text
packages/chain-client
```

Não duplicar manualmente layouts de conta em vários projetos.

## Etapa 4 — Backend

Implementar:

1. Configuração;
2. Migrations;
3. Autenticação de carteira;
4. Eventos;
5. Checkpoints;
6. QR rotativo;
7. Grants;
8. Construção e coassinatura de transações;
9. SSE;
10. Indexador;
11. Reconciliação.

Executar testes a cada módulo.

## Etapa 5 — Frontend

Implementar:

1. Conexão da carteira;
2. Autenticação;
3. Painel administrativo;
4. Display do QR;
5. Leitor;
6. Assinatura;
7. Progresso;
8. Dashboard;
9. Explorer Devnet.

Executar:

```text
/build-app
/test-ts
```

## Etapa 6 — E2E local

Executar o caminho vertical completo com validator local.

Nenhuma funcionalidade secundária deve ser priorizada enquanto o fluxo principal falhar.

## Etapa 7 — Auditoria antes do deploy

Executar:

```text
/test-and-fix
/audit-solana
/audit-infra
/diff-review
```

Corrigir problemas críticos e altos.

## Etapa 8 — Deploy na Devnet

Executar:

```text
/deploy devnet
```

Depois:

1. Atualizar `deployments/devnet.json`;
2. Atualizar variáveis do frontend e backend;
3. Gerar client usando o IDL implantado;
4. Verificar Program ID;
5. Executar smoke test;
6. Executar E2E Devnet;
7. Criar relatório.

## Etapa 9 — Revisão de produto

Executar:

```text
/product-review
```

Validar:

- UX móvel;
- Clareza da rede;
- Erros;
- Acessibilidade;
- Fluxo de carteira;
- QR expirado;
- Tempo de confirmação.

## Etapa 10 — Documentação e entrega

Executar:

```text
/write-docs
```

Entregar:

- README;
- Guia de instalação;
- Guia de operação;
- Arquitetura;
- Segurança;
- Manifesto Devnet;
- Relatório Devnet;
- Links do Explorer;
- Resultado dos testes;
- Limitações.

# 28. Instruções obrigatórias para a IA desenvolvedora

A IA deve:

1. Utilizar o `solana-ai-kit` como fluxo de desenvolvimento;
2. Informar quais agentes e skills foram utilizados;
3. Trabalhar em pequenas etapas verificáveis;
4. Não inventar APIs de bibliotecas;
5. Consultar documentação oficial e Context7;
6. Fixar versões no projeto;
7. Executar testes após cada alteração relevante;
8. Não ocultar falhas de compilação;
9. Não usar mocks no teste final de Devnet;
10. Manter Devnet como rede obrigatória;
11. Validar todas as contas no programa;
12. Manter segredos fora do Git;
13. Criar migrations;
14. Usar transações de banco quando necessário;
15. Implementar idempotência;
16. Documentar decisões em ADRs;
17. Não colocar dados pessoais on-chain;
18. Não permitir que o frontend conceda pontos;
19. Não confiar apenas no callback do navegador;
20. Não marcar presença antes da confirmação on-chain;
21. Não adicionar transferências financeiras ao check-in;
22. Não fazer deploy em Mainnet;
23. Não declarar sucesso sem evidências da Devnet;
24. Não apagar testes para fazer o pipeline passar;
25. Não enfraquecer validações para contornar um bug;
26. Não incluir seed phrase ou private key em logs, commits ou relatórios.

### Relatório após cada etapa

A IA deve informar:

```text
Objetivo da etapa
Agentes utilizados
Arquivos criados ou modificados
Comandos executados
Resultado dos testes
Falhas encontradas
Decisões arquiteturais
Pendências
Próximo passo
```

### Critério para utilizar “concluído”

A IA somente pode escrever que o MVP está concluído quando todos os itens abaixo forem verdadeiros:

```text
build do programa passou
build do frontend passou
build do backend passou
testes Rust passaram
testes TypeScript passaram
E2E local passou
programa está implantado na Devnet
smoke test Devnet passou
E2E full stack Devnet passou
check-in duplicado foi rejeitado
relatório Devnet foi gerado
auditorias foram executadas
README contém instruções reproduzíveis
```

### Prompt inicial recomendado para Claude Code

```text
Leia integralmente EventQuest_Especificacao_IA_Solana_AI_Kit.md.

Use obrigatoriamente o Solana AI Kit instalado neste repositório.
Execute primeiro /doctor e apresente somente problemas concretos encontrados.

Depois use /plan-feature e crie um plano vertical para entregar o EventQuest
full stack com Next.js, @solana/kit, Axum, PostgreSQL, Redis e programa Anchor.

Crie um agent team com:
- solana-architect;
- anchor-engineer;
- rust-backend-engineer;
- solana-frontend-engineer;
- solana-qa-engineer;
- devops-engineer.

O objetivo final é um app funcional e testado na Solana Devnet.
Toda a interface do app deve estar em inglês, usando locale en-US.
O visual deve ser dark mode, mobile-first, com laranja como cor de destaque.
Não implemente light mode no MVP.
Use QR preto sobre fundo branco para preservar a leitura.
Não considere mocks ou validator local como entrega final.
Não faça deploy em Mainnet.
Não exponha segredos.
Não implemente cNFT antes do fluxo principal estar concluído.

Implemente primeiro este caminho vertical:
criar evento → criar checkpoint → exibir QR rotativo → autenticar carteira
→ validar QR → preparar transação → assinar → registrar Attendance PDA
→ indexar → atualizar frontend → rejeitar duplicidade.

Ao final de cada etapa, apresente arquivos modificados, comandos,
resultados dos testes, limitações e próximo passo.
```

# 29. Definition of Done

Uma funcionalidade está concluída apenas quando:

- Compila;
- Possui validação;
- Possui testes positivos e negativos;
- Possui tratamento de erro;
- Possui logs seguros;
- Está documentada;
- Não expõe segredo;
- Funciona no ambiente local;
- Mantém compatibilidade com a stack fixada;
- Não introduz dependências sem justificativa;
- Possui critério de aceite demonstrável.

O MVP está concluído apenas quando:

- O frontend está funcional;
- Toda a interface está em inglês;
- O tema dark está aplicado globalmente;
- A identidade visual usa laranja como destaque;
- Os critérios de contraste e acessibilidade foram testados;
- A API Rust está funcional;
- PostgreSQL e Redis estão integrados;
- O programa está implantado na Solana Devnet;
- O Program ID está documentado;
- O QR rotativo funciona;
- A autenticação por carteira funciona;
- O participante assina a transação;
- A Attendance PDA é criada;
- A presença aparece no frontend após indexação;
- O check-in duplicado é rejeitado;
- O teste E2E local passa;
- O teste de fumaça na Devnet passa;
- O E2E full stack contra a Devnet passa;
- Auditorias foram executadas;
- O relatório de Devnet contém links verificáveis;
- O commit testado está identificado.

Não aceitar como entrega final:

- Apenas protótipo visual;
- Apenas programa Anchor;
- Apenas frontend;
- Apenas backend;
- Apenas validator local;
- Apenas transação simulada;
- Apenas testes mockados;
- Programa implantado sem integração com o site;
- Site funcional sem transação real na Devnet.

# 30. Possíveis evoluções

Após o MVP:

- Passaporte como cNFT Bubblegum V2;
- cNFT soulbound;
- Certificado verificável;
- Missões compostas;
- Ranking;
- Prêmios;
- Cupons;
- Solana Pay;
- Patrocínio de taxas;
- Embedded wallets;
- Aplicativo React Native;
- Notificações push;
- Modo offline limitado;
- Beacon Bluetooth;
- Integração com catracas;
- Integração com calendários;
- Painel multi-organização;
- Templates de eventos;
- Check-in por Solana Actions/Blinks;
- Solana Attestation System;
- API pública para parceiros.

---

# 31. Referências técnicas oficiais

## Solana AI Kit

- Repositório:  
  https://github.com/solanabr/solana-ai-kit

- Quick Start:  
  https://github.com/solanabr/solana-ai-kit/blob/main/QUICK-START.md

O projeto deve registrar o commit exato do kit usado na implementação.

## Solana

- Documentação geral:  
  https://solana.com/docs

- Desenvolvimento de frontend:  
  https://solana.com/docs/frontend

- Next.js e React:  
  https://solana.com/docs/frontend/nextjs-solana

- Assinatura e verificação de mensagens:  
  https://solana.com/developers/cookbook/wallets/sign-message

- Assinaturas em produção:  
  https://solana.com/docs/core/transactions/signing-in-production

- Solana Explorer Devnet:  
  https://explorer.solana.com/?cluster=devnet

## Anchor

- Documentação:  
  https://www.anchor-lang.com/docs

- Client TypeScript:  
  https://www.anchor-lang.com/docs/clients/typescript

- Testes:  
  https://www.anchor-lang.com/docs/testing

## Metaplex — somente fase posterior

- Bubblegum V2:  
  https://developers.metaplex.com/smart-contracts/bubblegum-v2

- SDK JavaScript:  
  https://developers.metaplex.com/smart-contracts/bubblegum-v2/sdk/javascript

- DAS e dados armazenados:  
  https://developers.metaplex.com/smart-contracts/bubblegum-v2/stored-nft-data

A IA deve preferir fontes oficiais e documentação correspondente às versões efetivamente instaladas.

# 32. Resumo para iniciar a implementação

## Resultado esperado

Entregar uma plataforma full stack composta por:

```text
Next.js frontend em inglês
+ dark mode com destaque laranja
+ carteira Solana
+ API Rust/Axum
+ PostgreSQL
+ Redis
+ QR rotativo
+ programa Anchor
+ indexador
+ testes
+ deploy na Solana Devnet
```

## Caminho vertical obrigatório

```text
1. Instalar e validar o Solana AI Kit;
2. Criar o monorepo;
3. Criar programa Anchor;
4. Criar evento na Devnet;
5. Criar checkpoint na Devnet;
6. Abrir display de QR rotativo;
7. Autenticar participante pela carteira;
8. Escanear e validar o QR;
9. Criar grant individual;
10. Backend preparar e coassinar a transação;
11. Participante assinar e enviar;
12. Programa criar Attendance PDA;
13. Indexador confirmar;
14. Frontend mostrar presença e pontos;
15. Segunda tentativa ser rejeitada;
16. Smoke test e E2E Devnet passarem;
17. Gerar relatório com links do Explorer.
```

## Sequência recomendada de comandos do kit

```text
/doctor
/setup-mcp
/plan-feature
/scaffold
/build-program
/build-app
/test-rust
/test-ts
/generate-idl-client
/test-and-fix
/profile-cu
/audit-solana
/audit-infra
/deploy devnet
/product-review
/diff-review
/write-docs
```

## Regra final

Não implementar antes do caminho principal:

- cNFT;
- Ranking;
- Marketplace;
- Token próprio;
- Recompensas financeiras;
- Integrações adicionais.

A primeira entrega deve provar que o frontend, o backend e a blockchain funcionam juntos na Solana Devnet.
