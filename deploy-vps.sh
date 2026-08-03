#!/bin/bash
# Deploy completo da Lista (dio-lista) para a VPS — espelho do LabNetCol.
#
# Uso:
#   ./deploy-vps.sh          → prod  (/opt/dio/dio-lista-prod, unit dio-lista-prod)
#   ./deploy-vps.sh prod     → idem
#   ./deploy-vps.sh teste    → teste (/opt/dio/dio-lista, unit dio-lista) — opcional
#
# PRIMEIRA VEZ (prod), na VPS:
#   sudo mkdir -p /opt/dio/dio-lista-prod/{dist,data/imgs}
#   sudo chown -R lnc:lnc /opt/dio/dio-lista-prod
#   cp .env.example → /opt/dio/dio-lista-prod/.env  (LABNETCOL_SECRET = portal!)
#   chmod 600 /opt/dio/dio-lista-prod/.env
#
# Nginx + certbot (uma vez):
#   ver deploy/nginx-lista.conf.example
#   sudo certbot --nginx -d lista.labnetcol.pt
#
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")" && pwd)"
VPS="lnc@2.56.212.222"
VPS_PORT="48499"
DIST="$ROOT_DIR/target/dx/dio-lista-web/release/web/public"
MODO="${1:-prod}"

case "$MODO" in
  prod|produção|producao)
    DEST="/opt/dio/dio-lista-prod"
    UNIT_SRC="$ROOT_DIR/dio-lista-prod.service"
    UNIT_DST="dio-lista-prod.service"
    BIN_NAME="lista_serv"
    ;;
  teste|test)
    DEST="/opt/dio/dio-lista"
    UNIT_SRC="$ROOT_DIR/dio-lista.service"
    UNIT_DST="dio-lista.service"
    BIN_NAME="lista_serv"
    ;;
  *)
    echo "Uso: $0 [prod|teste]" >&2
    exit 1
    ;;
esac

SSH="ssh -p $VPS_PORT $VPS"
SSHT="ssh -t -p $VPS_PORT $VPS"

echo "==> Modo: $MODO → $DEST (unit $UNIT_DST)"

# ── 0. Pastas na VPS ──────────────────────────────────────────
echo "==> Garantir pastas na VPS (dist, data/imgs)"
$SSH "mkdir -p $DEST/dist $DEST/data/imgs && chmod -R u+rwX $DEST/dist $DEST/data"

# ── 1. Build frontend WASM ────────────────────────────────────
echo "==> Build frontend Dioxus/WASM"
cd "$ROOT_DIR/frontend-web"
dx build --platform web --release --debug-symbols false
cd "$ROOT_DIR"

# ── 2. Build servidor ─────────────────────────────────────────
echo "==> Build servidor Axum"
cargo build -p dio-lista-server --release

# ── 3. Enviar binário ─────────────────────────────────────────
echo "==> Enviar binário"
scp -P "$VPS_PORT" "$ROOT_DIR/target/release/lista_serv" "$VPS:/tmp/lista_serv"
$SSH "mv /tmp/lista_serv $DEST/$BIN_NAME && chmod +x $DEST/$BIN_NAME"

# ── 4. Enviar frontend ────────────────────────────────────────
echo "==> Enviar frontend WASM"
if [[ ! -d "$DIST" ]]; then
    echo "ERRO: DIST inexistente: $DIST" >&2
    echo "Correu o dx build? (passo 1)" >&2
    exit 1
fi
if ! find "$DIST" -name '*.wasm' | grep -q .; then
    echo "ERRO: sem ficheiros .wasm em $DIST — build incompleto." >&2
    exit 1
fi
echo "    origem: $DIST ($(du -sh "$DIST" | cut -f1))"
echo "    destino: $VPS:$DEST/dist/"
rsync -az --info=progress2 -e "ssh -p $VPS_PORT" --delete "$DIST/" "$VPS:$DEST/dist/"
echo "    rsync OK"
$SSH "ls -la $DEST/dist/ | head -5; echo -n '    wasm na VPS: '; find $DEST/dist -name '*.wasm' | wc -l"

# ── 5. Seed JSON (opcional, se existir localmente) ────────────
SEED_LOCAL="$ROOT_DIR/seed/catalogo_base.json"
if [[ -f "$SEED_LOCAL" ]]; then
    echo "==> Enviar seed/catalogo_base.json"
    $SSH "mkdir -p $DEST/seed"
    scp -P "$VPS_PORT" "$SEED_LOCAL" "$VPS:$DEST/seed/catalogo_base.json"
fi

# ── 6. Unit systemd ───────────────────────────────────────────
echo "==> Actualizar serviço systemd e reiniciar (pede password sudo)"
scp -P "$VPS_PORT" "$UNIT_SRC" "$VPS:/tmp/$UNIT_DST"
$SSHT "sudo mv /tmp/$UNIT_DST /etc/systemd/system/$UNIT_DST \
    && sudo systemctl daemon-reload \
    && sudo systemctl enable $UNIT_DST \
    && sudo systemctl restart $UNIT_DST \
    && sudo systemctl status $UNIT_DST --no-pager -l"

echo ""
echo "Deploy Lista concluído → $DEST"
echo "  Local:  curl -sI http://127.0.0.1:8088/ | head -3"
echo "  Público: curl -sI https://lista.labnetcol.pt/ | head -5"
echo "  (nginx: deploy/nginx-lista.conf.example + certbot -d lista.labnetcol.pt)"
