//! Listas de compras + itens.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use lista_comum::{
    preco_unitario, ArtigoReg, Compra, CompraHistorico, CompraReg, Ida, IdaReg, ItemLista,
    ItemListaDto, ItemListaReg, ListaCompra, ListaCompraReg, LojaReg, PedidoItem, PedidoItemPatch,
    PedidoLista, PedidoNovaIda, RefAlteracaoReg, TAM_NOME, N_HISTORICO_PRECOS, UTILIZADOR_BASE,
};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct TotaisLista {
    pub n_produtos: usize,
    pub n_comprados: usize,
    /// Soma dos preços registados nos ✓ desta lista (ida actual).
    pub total_cent: u32,
    pub n_com_preco: usize,
    /// Estimativa dos por comprar (preço ref. × qtd).
    pub estimado_cent: u32,
    pub n_estimado: usize,
    /// Estimativa de toda a lista (todos os itens × qtd) — aba Listas.
    pub estimado_lista_cent: u32,
    pub n_estimado_lista: usize,
}
use mcs_bd2::{bd_alterar, bd_gravar, bd_ler_dados, bd_listar, bd_remover};

use crate::estado::AppState;
use crate::rotas::auth_rota::verificar_token;

macro_rules! exige {
    ($headers:expr, $state:expr) => {
        match verificar_token(&$headers, &$state.jwt_secret) {
            Some(s) => s,
            None => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({"erro":"Não autenticado. Entra pelo LabNetCol."})),
                )
                    .into_response()
            }
        }
    };
}

macro_rules! bd {
    ($state:expr, $nome:expr) => {
        match $state.bd.get($nome) {
            Some(b) => b.clone(),
            None => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({"erro":"Serviço indisponível"})),
                )
                    .into_response()
            }
        }
    };
}

async fn lista_do_dono(
    state: &AppState,
    lista_id: u64,
    dono: u64,
) -> Result<ListaCompraReg, (StatusCode, Json<serde_json::Value>)> {
    let bd = state
        .bd
        .get("lista")
        .cloned()
        .ok_or((
            StatusCode::SERVICE_UNAVAILABLE,
            Json(serde_json::json!({"erro":"Serviço indisponível"})),
        ))?;
    let Some(reg) = bd_ler_dados::<ListaCompraReg>(bd, lista_id)
        .await
        .ok()
        .flatten()
    else {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"erro":"Lista não encontrada."})),
        ));
    };
    if reg.utilizador != dono {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"erro":"Esta lista não é tua."})),
        ));
    }
    Ok(reg)
}

/// Ida mais recente desta lista (utilizador).
async fn ida_activa_lista(state: &AppState, lista_id: u64, dono: u64) -> Option<Ida> {
    let bd = state.bd.get("ida")?.clone();
    let mut melhor: Option<Ida> = None;
    for (n, r) in bd_listar::<IdaReg>(bd).await.unwrap_or_default() {
        if r.utilizador != dono || r.lista_id != lista_id {
            continue;
        }
        let i = r.to_ida(n);
        let trocar = match &melhor {
            None => true,
            Some(m) => {
                i.iniciada_em > m.iniciada_em
                    || (i.iniciada_em == m.iniciada_em && i.n_reg > m.n_reg)
            }
        };
        if trocar {
            melhor = Some(i);
        }
    }
    melhor
}

/// Último preço unitário por artigo (compras do utilizador com preço).
async fn mapa_ultimo_unitario(
    state: &AppState,
    utilizador: u64,
) -> std::collections::HashMap<u64, (u64, u32)> {
    let mut ultimo = std::collections::HashMap::new();
    let Some(bd_c) = state.bd.get("compra") else {
        return ultimo;
    };
    for (_, r) in bd_listar::<CompraReg>(bd_c.clone())
        .await
        .unwrap_or_default()
    {
        if r.utilizador != utilizador || r.preco_cent == 0 {
            continue;
        }
        let unit = preco_unitario(r.preco_cent, r.qtd);
        if unit == 0 {
            continue;
        }
        let entry = ultimo.entry(r.artigo_id).or_insert((0, 0));
        if r.comprado_em >= entry.0 {
            *entry = (r.comprado_em, unit);
        }
    }
    ultimo
}

fn unitario_ref(art: Option<&ArtigoReg>, ultimo: &std::collections::HashMap<u64, (u64, u32)>, artigo_id: u64) -> u32 {
    // Preferir último preço do utilizador; senão referência do artigo.
    if let Some((_, u)) = ultimo.get(&artigo_id) {
        if *u > 0 {
            return *u;
        }
    }
    art.map(|a| a.preco_referencia).unwrap_or(0)
}

fn item_dto(
    n_reg: u64,
    r: &ItemListaReg,
    nome: String,
    imag: String,
    secao: u8,
    preco_ref_cent: u32,
) -> ItemListaDto {
    ItemListaDto {
        n_reg,
        lista_id: r.lista_id,
        artigo_id: r.artigo_id,
        nome,
        imag,
        secao,
        qtd: r.qtd,
        unidade: r.unidade,
        estado: r.estado,
        preco_ref_cent,
    }
}

/// GET /api/listas/{id}/ida — ida activa (ou 404).
pub async fn obter_ida(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    let sess = exige!(headers, state);
    if let Err(e) = lista_do_dono(&state, id, sess.labnetcol_id).await {
        return e.into_response();
    }
    match ida_activa_lista(&state, id, sess.labnetcol_id).await {
        Some(i) => (StatusCode::OK, Json(i)).into_response(),
        None => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"erro":"Sem ida activa."})),
        )
            .into_response(),
    }
}

/// POST /api/listas/{id}/ida — começa nova ida (Registado a zero).
pub async fn nova_ida(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
    Json(pedido): Json<PedidoNovaIda>,
) -> impl IntoResponse {
    let sess = exige!(headers, state);
    if let Err(e) = lista_do_dono(&state, id, sess.labnetcol_id).await {
        return e.into_response();
    }
    let bd = bd!(state, "ida");
    let ida = Ida::nova(sess.labnetcol_id, id, pedido.loja_id);
    match bd_gravar(bd, IdaReg::from_ida(&ida)).await {
        Ok(n_reg) => {
            let mut out = ida;
            out.n_reg = n_reg;
            (StatusCode::CREATED, Json(out)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"erro": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /api/listas
pub async fn listar_listas(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let sess = exige!(headers, state);
    let dono = sess.labnetcol_id;
    if dono == 0 {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"erro":"Sessão inválida."})),
        )
            .into_response();
    }
    let bd = bd!(state, "lista");
    let mut lista: Vec<ListaCompra> = bd_listar::<ListaCompraReg>(bd)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, r)| r.utilizador == dono)
        .map(|(n, r)| r.to_lista(n))
        .collect();
    lista.sort_by(|a, b| b.criada_em.cmp(&a.criada_em));
    (StatusCode::OK, Json(lista)).into_response()
}

/// POST /api/listas
pub async fn criar_lista(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(pedido): Json<PedidoLista>,
) -> impl IntoResponse {
    let sess = exige!(headers, state);
    let dono = sess.labnetcol_id;
    if dono == 0 {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"erro":"Sessão inválida."})),
        )
            .into_response();
    }
    let nome = pedido.nome.trim();
    if nome.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"erro":"Indica o nome da lista."})),
        )
            .into_response();
    }
    if nome.as_bytes().len() > TAM_NOME {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"erro": format!("Nome: máximo {TAM_NOME} bytes.")})),
        )
            .into_response();
    }
    let l = ListaCompra::nova(dono, nome);
    let reg = ListaCompraReg::from_lista(&l);
    let bd = bd!(state, "lista");
    match bd_gravar(bd, reg).await {
        Ok(n) => (StatusCode::CREATED, Json(reg.to_lista(n))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"erro": e.to_string()})),
        )
            .into_response(),
    }
}

/// DELETE /api/listas/{id} — apaga lista e os seus itens.
pub async fn remover_lista(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    let sess = exige!(headers, state);
    if let Err(e) = lista_do_dono(&state, id, sess.labnetcol_id).await {
        return e.into_response();
    }
    let bd_itens = bd!(state, "item");
    let itens = bd_listar::<ItemListaReg>(bd_itens.clone())
        .await
        .unwrap_or_default();
    for (n, r) in itens {
        if r.lista_id == id {
            let _ = bd_remover(bd_itens.clone(), n).await;
        }
    }
    let bd = bd!(state, "lista");
    match bd_remover(bd, id).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"erro": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /api/listas/{id}/itens
pub async fn listar_itens(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    let sess = exige!(headers, state);
    if let Err(e) = lista_do_dono(&state, id, sess.labnetcol_id).await {
        return e.into_response();
    }
    let bd_itens = bd!(state, "item");
    let bd_art = bd!(state, "artigo");
    let ultimo = mapa_ultimo_unitario(&state, sess.labnetcol_id).await;
    let mut out = Vec::<ItemListaDto>::new();
    for (n, r) in bd_listar::<ItemListaReg>(bd_itens)
        .await
        .unwrap_or_default()
    {
        if r.lista_id != id {
            continue;
        }
        let art = bd_ler_dados::<ArtigoReg>(bd_art.clone(), r.artigo_id)
            .await
            .ok()
            .flatten();
        let preco_ref_cent = unitario_ref(art.as_ref(), &ultimo, r.artigo_id);
        let (nome, imag, secao) = match art {
            Some(a) => {
                let a = a.to_artigo(r.artigo_id);
                (a.nome, a.imag, a.secao)
            }
            None => ("(apagado)".into(), String::new(), 0),
        };
        out.push(item_dto(n, &r, nome, imag, secao, preco_ref_cent));
    }
    out.sort_by(|a, b| {
        a.estado
            .cmp(&b.estado)
            .then_with(|| a.secao.cmp(&b.secao))
            .then_with(|| a.nome.to_lowercase().cmp(&b.nome.to_lowercase()))
    });
    (StatusCode::OK, Json(out)).into_response()
}

/// GET /api/listas/{id}/totais — produtos, valor registado nesta ida, estimativa das compras.
pub async fn totais_lista(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    let sess = exige!(headers, state);
    if let Err(e) = lista_do_dono(&state, id, sess.labnetcol_id).await {
        return e.into_response();
    }
    let bd_itens = bd!(state, "item");
    let mut n_produtos = 0usize;
    let mut n_comprados = 0usize;
    let mut itens_lista = Vec::<ItemListaReg>::new();
    for (_, r) in bd_listar::<ItemListaReg>(bd_itens)
        .await
        .unwrap_or_default()
    {
        if r.lista_id != id {
            continue;
        }
        // qtd 0 = na lista-geral mas «desligado» nesta altura
        if r.qtd > 0 {
            n_produtos += 1;
            if r.estado == 1 {
                n_comprados += 1;
            }
        }
        itens_lista.push(r);
    }

    // Registado = só compras desta ida (desde iniciada_em). Sem ida → 0.
    let desde_ida = ida_activa_lista(&state, id, sess.labnetcol_id)
        .await
        .map(|i| i.iniciada_em);

    let ultimo = mapa_ultimo_unitario(&state, sess.labnetcol_id).await;
    let mut total_cent = 0u32;
    let mut n_com_preco = 0usize;
    if let Some(bd_c) = state.bd.get("compra") {
        for (_, r) in bd_listar::<CompraReg>(bd_c.clone())
            .await
            .unwrap_or_default()
        {
            if r.utilizador != sess.labnetcol_id || r.preco_cent == 0 {
                continue;
            }
            if r.lista_id == id {
                let na_ida = desde_ida
                    .map(|d| r.comprado_em >= d)
                    .unwrap_or(false);
                if na_ida {
                    total_cent = total_cent.saturating_add(r.preco_cent);
                    n_com_preco += 1;
                }
            }
        }
    }

    let mut estimado_cent = 0u32;
    let mut n_estimado = 0usize;
    let mut estimado_lista_cent = 0u32;
    let mut n_estimado_lista = 0usize;
    let bd_art = state.bd.get("artigo").cloned();
    for r in &itens_lista {
        if r.qtd == 0 {
            continue;
        }
        let mut unit = 0u32;
        if let Some(ref bd_a) = bd_art {
            if let Ok(Some(ar)) = bd_ler_dados::<ArtigoReg>(bd_a.clone(), r.artigo_id).await {
                unit = unitario_ref(Some(&ar), &ultimo, r.artigo_id);
            }
        }
        if unit == 0 {
            unit = ultimo.get(&r.artigo_id).map(|(_, u)| *u).unwrap_or(0);
        }
        if unit == 0 {
            continue;
        }
        let linha = unit.saturating_mul(u32::from(r.qtd));
        estimado_lista_cent = estimado_lista_cent.saturating_add(linha);
        n_estimado_lista += 1;
        if r.estado == 0 {
            estimado_cent = estimado_cent.saturating_add(linha);
            n_estimado += 1;
        }
    }

    (
        StatusCode::OK,
        Json(TotaisLista {
            n_produtos,
            n_comprados,
            total_cent,
            n_com_preco,
            estimado_cent,
            n_estimado,
            estimado_lista_cent,
            n_estimado_lista,
        }),
    )
        .into_response()
}

/// POST /api/listas/{id}/itens
pub async fn adicionar_item(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
    Json(pedido): Json<PedidoItem>,
) -> impl IntoResponse {
    let sess = exige!(headers, state);
    let dono = sess.labnetcol_id;
    if let Err(e) = lista_do_dono(&state, id, dono).await {
        return e.into_response();
    }
    let bd_art = bd!(state, "artigo");
    let Some(art_reg) = bd_ler_dados::<ArtigoReg>(bd_art, pedido.artigo_id)
        .await
        .ok()
        .flatten()
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"erro":"Produto não encontrado."})),
        )
            .into_response();
    };
    let artigo = art_reg.to_artigo(pedido.artigo_id);
    let permitido =
        artigo.utilizador == UTILIZADOR_BASE || artigo.utilizador == dono;
    if !permitido {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"erro":"Só podes usar o catálogo base ou os teus produtos."})),
        )
            .into_response();
    }

    // Se já existe o mesmo artigo na lista, soma qtd
    let bd_itens = bd!(state, "item");
    let qtd_add = if pedido.qtd == 0 { 1 } else { pedido.qtd };
    for (n, r) in bd_listar::<ItemListaReg>(bd_itens.clone())
        .await
        .unwrap_or_default()
    {
        if r.lista_id == id && r.artigo_id == pedido.artigo_id {
            let mut r2 = r;
            r2.qtd = r2.qtd.saturating_add(qtd_add);
            match bd_alterar(bd_itens, r2, n).await {
                Ok(_) => {
                    let dto = ItemListaDto {
                        n_reg: n,
                        lista_id: id,
                        artigo_id: artigo.n_reg,
                        nome: artigo.nome,
                        imag: artigo.imag,
                        secao: artigo.secao,
                        qtd: r2.qtd,
                        unidade: r2.unidade,
                        estado: r2.estado,
                        preco_ref_cent: artigo.preco_referencia,
                    };
                    return (StatusCode::OK, Json(dto)).into_response();
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"erro": e.to_string()})),
                    )
                        .into_response()
                }
            }
        }
    }

    let item = ItemLista::novo(id, &artigo, qtd_add);
    let reg = ItemListaReg::from_item(&item);
    match bd_gravar(bd_itens, reg).await {
        Ok(n) => {
            let dto = ItemListaDto {
                n_reg: n,
                lista_id: id,
                artigo_id: artigo.n_reg,
                nome: artigo.nome,
                imag: artigo.imag,
                secao: artigo.secao,
                qtd: reg.qtd,
                unidade: reg.unidade,
                estado: reg.estado,
                preco_ref_cent: artigo.preco_referencia,
            };
            (StatusCode::CREATED, Json(dto)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"erro": e.to_string()})),
        )
            .into_response(),
    }
}

/// PUT /api/itens/{id}
pub async fn editar_item(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
    Json(pedido): Json<PedidoItemPatch>,
) -> impl IntoResponse {
    let sess = exige!(headers, state);
    let bd_itens = bd!(state, "item");
    let Some(mut reg) = bd_ler_dados::<ItemListaReg>(bd_itens.clone(), id)
        .await
        .ok()
        .flatten()
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"erro":"Item não encontrado."})),
        )
            .into_response();
    };
    if let Err(e) = lista_do_dono(&state, reg.lista_id, sess.labnetcol_id).await {
        return e.into_response();
    }
    let estado_antes = reg.estado;
    if let Some(q) = pedido.qtd {
        reg.qtd = q; // 0 = desligado na lista-geral
    }
    if let Some(e) = pedido.estado {
        reg.estado = if e == 1 { 1 } else { 0 };
    }
    match bd_alterar(bd_itens.clone(), reg, id).await {
        Ok(_) => {
            // Ao marcar comprado: histórico (loja + preço; 0 → estimativa ref × qtd).
            if estado_antes != 1 && reg.estado == 1 && reg.qtd > 0 {
                let loja_id = pedido.loja_id.unwrap_or(0);
                let mut preco_cent = pedido.preco_cent.unwrap_or(0);
                if preco_cent == 0 {
                    let ultimo = mapa_ultimo_unitario(&state, sess.labnetcol_id).await;
                    let art_reg = if let Some(bd_a) = state.bd.get("artigo") {
                        bd_ler_dados::<ArtigoReg>(bd_a.clone(), reg.artigo_id)
                            .await
                            .ok()
                            .flatten()
                    } else {
                        None
                    };
                    let unit = unitario_ref(art_reg.as_ref(), &ultimo, reg.artigo_id);
                    preco_cent = unit.saturating_mul(u32::from(reg.qtd));
                }
                let compra = Compra::nova(
                    sess.labnetcol_id,
                    reg.artigo_id,
                    reg.lista_id,
                    reg.qtd,
                    reg.unidade,
                    preco_cent,
                    loja_id,
                );
                if let Some(bd_c) = state.bd.get("compra") {
                    if let Err(e) = bd_gravar(bd_c.clone(), CompraReg::from_compra(&compra)).await {
                        log::warn!("compra não gravada (item {id}): {e}");
                    } else if preco_cent > 0 {
                        // Mantém preço unitário no artigo (ref rápida); o job diário
                        // recalcula a moda global depois. Evita zerar no ↩ / reabrir.
                        let unit = preco_unitario(preco_cent, reg.qtd);
                        if unit > 0 {
                            if let Some(bd_a) = state.bd.get("artigo") {
                                if let Ok(Some(mut art)) =
                                    bd_ler_dados::<ArtigoReg>(bd_a.clone(), reg.artigo_id).await
                                {
                                    if art.preco_referencia != unit {
                                        art.preco_referencia = unit;
                                        if let Err(e) =
                                            bd_alterar(bd_a.clone(), art, reg.artigo_id).await
                                        {
                                            log::warn!(
                                                "preco_referencia artigo {}: {e}",
                                                reg.artigo_id
                                            );
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Ao desfazer ✓: tira a compra mais recente deste artigo na ida actual.
            // Antes de apagar, se o artigo ainda não tem ref, guarda o unitário dessa compra.
            if estado_antes == 1 && reg.estado == 0 {
                if let Some(bd_c) = state.bd.get("compra") {
                    let desde = ida_activa_lista(&state, reg.lista_id, sess.labnetcol_id)
                        .await
                        .map(|i| i.iniciada_em)
                        .unwrap_or(0);
                    let mut melhor: Option<(u64, u64, u32, u16)> = None; // n_reg, ts, preco, qtd
                    for (n, r) in bd_listar::<CompraReg>(bd_c.clone())
                        .await
                        .unwrap_or_default()
                    {
                        if r.utilizador != sess.labnetcol_id
                            || r.lista_id != reg.lista_id
                            || r.artigo_id != reg.artigo_id
                            || r.comprado_em < desde
                        {
                            continue;
                        }
                        let trocar = match melhor {
                            None => true,
                            Some((_, ts, _, _)) => r.comprado_em >= ts,
                        };
                        if trocar {
                            melhor = Some((n, r.comprado_em, r.preco_cent, r.qtd));
                        }
                    }
                    if let Some((n_compra, _, preco_linha, qtd_c)) = melhor {
                        let unit = preco_unitario(preco_linha, qtd_c);
                        if unit > 0 {
                            if let Some(bd_a) = state.bd.get("artigo") {
                                if let Ok(Some(mut art)) =
                                    bd_ler_dados::<ArtigoReg>(bd_a.clone(), reg.artigo_id).await
                                {
                                    if art.preco_referencia == 0 {
                                        art.preco_referencia = unit;
                                        let _ = bd_alterar(bd_a.clone(), art, reg.artigo_id).await;
                                    }
                                }
                            }
                        }
                        if let Err(e) = bd_remover(bd_c.clone(), n_compra).await {
                            log::warn!("compra {n_compra} não removida ao desfazer item {id}: {e}");
                        }
                    }
                }
            }
            let bd_art = bd!(state, "artigo");
            let art_reg = bd_ler_dados::<ArtigoReg>(bd_art, reg.artigo_id)
                .await
                .ok()
                .flatten();
            let ultimo = mapa_ultimo_unitario(&state, sess.labnetcol_id).await;
            let preco_ref_cent = unitario_ref(art_reg.as_ref(), &ultimo, reg.artigo_id);
            let art = art_reg.map(|a| a.to_artigo(reg.artigo_id));
            let dto = ItemListaDto {
                n_reg: id,
                lista_id: reg.lista_id,
                artigo_id: reg.artigo_id,
                nome: art.as_ref().map(|a| a.nome.clone()).unwrap_or_else(|| "?".into()),
                imag: art.as_ref().map(|a| a.imag.clone()).unwrap_or_default(),
                secao: art.as_ref().map(|a| a.secao).unwrap_or(0),
                qtd: reg.qtd,
                unidade: reg.unidade,
                estado: reg.estado,
                preco_ref_cent,
            };
            (StatusCode::OK, Json(dto)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"erro": e.to_string()})),
        )
            .into_response(),
    }
}

/// DELETE /api/itens/{id}
pub async fn remover_item(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    let sess = exige!(headers, state);
    let bd_itens = bd!(state, "item");
    let Some(reg) = bd_ler_dados::<ItemListaReg>(bd_itens.clone(), id)
        .await
        .ok()
        .flatten()
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"erro":"Item não encontrado."})),
        )
            .into_response();
    };
    if let Err(e) = lista_do_dono(&state, reg.lista_id, sess.labnetcol_id).await {
        return e.into_response();
    }
    match bd_remover(bd_itens, id).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"erro": e.to_string()})),
        )
            .into_response(),
    }
}

/// GET /api/artigos/{id}/historico-precos — últimas compras deste produto (loja + preço).
pub async fn historico_precos_artigo(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(artigo_id): Path<u64>,
) -> impl IntoResponse {
    let sess = exige!(headers, state);
    let Some(bd_c) = state.bd.get("compra") else {
        return (StatusCode::OK, Json(Vec::<CompraHistorico>::new())).into_response();
    };
    let mut nomes_loja: std::collections::HashMap<u64, String> = std::collections::HashMap::new();
    if let Some(bd_l) = state.bd.get("loja") {
        for (n, r) in bd_listar::<LojaReg>(bd_l.clone())
            .await
            .unwrap_or_default()
        {
            let l = r.to_loja(n);
            if !l.nome.is_empty() {
                nomes_loja.insert(n, l.nome);
            }
        }
    }
    let mut pares: Vec<(u64, CompraReg)> = bd_listar::<CompraReg>(bd_c.clone())
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, r)| {
            r.utilizador == sess.labnetcol_id && r.artigo_id == artigo_id && r.preco_cent > 0
        })
        .collect();
    pares.sort_by(|a, b| b.1.comprado_em.cmp(&a.1.comprado_em).then(b.0.cmp(&a.0)));
    let out: Vec<CompraHistorico> = pares
        .into_iter()
        .take(N_HISTORICO_PRECOS)
        .map(|(n, r)| {
            let loja = if r.loja_id == 0 {
                "Sem loja".into()
            } else {
                nomes_loja
                    .get(&r.loja_id)
                    .cloned()
                    .unwrap_or_else(|| format!("Loja #{}", r.loja_id))
            };
            CompraHistorico {
                n_reg: n,
                loja,
                loja_id: r.loja_id,
                preco_cent: r.preco_cent,
                preco_unit_cent: preco_unitario(r.preco_cent, r.qtd),
                qtd: r.qtd,
                comprado_em: r.comprado_em,
            }
        })
        .collect();
    (StatusCode::OK, Json(out)).into_response()
}

#[derive(Clone, Debug, Serialize)]
pub struct RefAlteracaoDto {
    pub n_reg: u64,
    pub artigo_id: u64,
    pub artigo: String,
    pub preco_antes: u32,
    pub preco_depois: u32,
    pub lojas: Vec<String>,
    pub alterado_em: u64,
}

/// GET /api/ref-alteracoes — últimas alterações do preço de referência (job diário).
pub async fn listar_ref_alteracoes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let _sess = exige!(headers, state);
    let bd_r = bd!(state, "ref_alt");
    let mut nomes_art: std::collections::HashMap<u64, String> = std::collections::HashMap::new();
    if let Some(bd_a) = state.bd.get("artigo") {
        for (n, r) in bd_listar::<ArtigoReg>(bd_a.clone())
            .await
            .unwrap_or_default()
        {
            let a = r.to_artigo(n);
            if !a.nome.is_empty() {
                nomes_art.insert(n, a.nome);
            }
        }
    }
    let mut nomes_loja: std::collections::HashMap<u64, String> = std::collections::HashMap::new();
    if let Some(bd_l) = state.bd.get("loja") {
        for (n, r) in bd_listar::<LojaReg>(bd_l.clone())
            .await
            .unwrap_or_default()
        {
            let l = r.to_loja(n);
            if !l.nome.is_empty() {
                nomes_loja.insert(n, l.nome);
            }
        }
    }
    let mut pares = bd_listar::<RefAlteracaoReg>(bd_r)
        .await
        .unwrap_or_default();
    pares.sort_by(|a, b| b.1.alterado_em.cmp(&a.1.alterado_em).then(b.0.cmp(&a.0)));
    let out: Vec<RefAlteracaoDto> = pares
        .into_iter()
        .take(80)
        .map(|(n, r)| {
            let alt = r.to_alt(n);
            let artigo = nomes_art
                .get(&alt.artigo_id)
                .cloned()
                .unwrap_or_else(|| format!("#{}", alt.artigo_id));
            let lojas: Vec<String> = alt
                .loja_ids
                .iter()
                .filter(|&&id| id > 0)
                .map(|id| {
                    nomes_loja
                        .get(id)
                        .cloned()
                        .unwrap_or_else(|| format!("Loja #{id}"))
                })
                .collect();
            RefAlteracaoDto {
                n_reg: alt.n_reg,
                artigo_id: alt.artigo_id,
                artigo,
                preco_antes: alt.preco_antes,
                preco_depois: alt.preco_depois,
                lojas,
                alterado_em: alt.alterado_em,
            }
        })
        .collect();
    (StatusCode::OK, Json(out)).into_response()
}
