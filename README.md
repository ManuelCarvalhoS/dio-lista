# Lista

App de compras no hipermercado — **Rust + Dioxus**, independente do portal LabNetCol (padrão Lancer).

## Arquitectura

| Peça | Pasta | Papel |
|------|--------|--------|
| App Android / local | `src/` (crate `dio-lista`) | mcs_bd2 no telemóvel |
| Tipos partilhados | `lista_comum/` | Artigo, labels, pedidos API |
| Server web | `server/` → `lista_serv` | Axum + **mcs_bd2** + SSO |
| Frontend web | `frontend-web/` | UI; entra via LabNetCol SSO |

Identidade web (fase testes / família / visibilidade):
- **Entrada:** Lista directa — nome → lista própria (`POST /api/auth/dev`). Sem botão LabNetCol na UI da Lista (ainda).
- **Portal:** atalho/tile Lista no LabNetCol (ok).
- **Mais tarde:** botão LabNetCol no menu da Lista + eventual entrada em 2 modos; regra Cursor `lista-labnetcol-atalho`.
- **Produção:** `LISTA_DEV_LOGIN=0` e SSO quando fizer sentido.

## Correr — só Lista (teste rápido)

```bash
cd /home/nel2/prog2/Cur/dio-lista
export LISTA_PORT=8088
export LISTA_DATA_DIR=./server/data
export LISTA_DIST=./frontend-web/dist
cargo run -p dio-lista-server
# abre http://localhost:8088 → «Entrar (teste local)»
```

Hot-reload UI (opcional, 2.º terminal):

```bash
cd frontend-web && dx serve --port 8090
# API continua em :8088
```

## Correr — server + web (SSO LabNetCol)

```bash
# terminal 1 — API + ficheiros estáticos
cd /home/nel2/prog2/Cur/dio-lista
export LISTA_PORT=8088
export LISTA_DATA_DIR=./server/data
export LISTA_DIST=./frontend-web/dist
export LABNETCOL_SECRET=labnetcol-sso-dev-secret   # igual ao portal
cargo run -p dio-lista-server

# terminal 2 — build WASM (ou dx serve em :8088 só em dev)
cd frontend-web && dx build --release
# copiar public → dist se o dx não escrever lá
```

Portal (área autenticada) → tile **Lista** → `http://localhost:8088/sso?token=…`.

Acesso directo sem sessão → página “Entra pelo LabNetCol”.

## Correr — Android (local, sem server)

```bash
cd /home/nel2/prog2/Cur/dio-lista
dx serve --android --target aarch64-linux-android
```

## Env (server)

| Var | Default |
|-----|---------|
| `LISTA_PORT` | `8088` |
| `LISTA_DATA_DIR` | `./data` |
| `LISTA_DIST` | `../frontend-web/dist` |
| `LISTA_JWT_SECRET` | `lista-jwt-dev` |
| `LISTA_DEV_LOGIN` | **on** por omissão; `0` desliga (produção) |
| `LABNETCOL_SECRET` | `labnetcol-sso-dev-secret` |
| `LABNETCOL_FRONTEND_URL` | `http://localhost:8080` |

## Produção (VPS, espelho LabNetCol)

Mesmo padrão que `/opt/dio/dio-labnetcol-prod/`:

```bash
# 1ª vez na VPS
sudo mkdir -p /opt/dio/dio-lista-prod/{dist,data/imgs}
sudo chown -R lnc:lnc /opt/dio/dio-lista-prod
# .env a partir de .env.example — LABNETCOL_SECRET **igual** ao portal
chmod 600 /opt/dio/dio-lista-prod/.env

# nginx: copiar deploy/nginx-lista.conf.example → sites-available
# sudo certbot --nginx -d lista.labnetcol.pt

# na máquina de build
cd /home/nel2/prog2/Cur/dio-lista
./deploy-vps.sh          # ou ./deploy-vps.sh prod
```

| Peça | Valor |
|------|--------|
| Pasta | `/opt/dio/dio-lista-prod` |
| Unit | `dio-lista-prod.service` |
| Binário | `lista_serv` |
| Porta | `8088` |
| DNS | `lista.labnetcol.pt` |

Job diário de preços: loop interno no server. Android sync: **depois** do web estável.

## API (JWT sessão Lista após SSO)

- `POST /api/sso/labnetcol` `{ "token": "<sso>" }`
- `GET /api/artigos`, `POST /api/artigo`, …
- `GET /imgs/…` imagens estáticas
