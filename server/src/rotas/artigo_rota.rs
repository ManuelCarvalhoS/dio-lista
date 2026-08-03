//! Catálogo de produtos — passo 1: CRUD base pelo admin.

use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use lista_comum::{
    agora_unix, nome_ficheiro_imag, Artigo, ArtigoReg, PedidoArtigo, TAM_IMAG, TAM_NOME,
    UTILIZADOR_BASE,
};
use mcs_bd2::{bd_alterar, bd_gravar, bd_ler_dados, bd_listar, bd_remover};
use serde::Deserialize;
use std::path::PathBuf;

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

macro_rules! exige_admin {
    ($headers:expr, $state:expr) => {{
        let s = exige!($headers, $state);
        if !s.admin {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({"erro":"Só o administrador gere o catálogo base."})),
            )
                .into_response();
        }
        s
    }};
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

fn data_dir() -> PathBuf {
    PathBuf::from(std::env::var("LISTA_DATA_DIR").unwrap_or_else(|_| "./data".into()))
}

fn imgs_dir() -> PathBuf {
    data_dir().join("imgs")
}

/// GET /api/catalogo — produtos base (todos autenticados).
pub async fn listar_catalogo(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let _sess = exige!(headers, state);
    let bd = bd!(state, "artigo");
    let mut lista: Vec<Artigo> = bd_listar::<ArtigoReg>(bd)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, r)| r.utilizador == UTILIZADOR_BASE)
        .map(|(n, r)| r.to_artigo(n))
        .collect();
    lista.sort_by(|a, b| {
        a.secao
            .cmp(&b.secao)
            .then_with(|| a.nome.to_lowercase().cmp(&b.nome.to_lowercase()))
    });
    (StatusCode::OK, Json(lista)).into_response()
}

/// POST /api/catalogo — criar produto base (admin).
pub async fn criar_catalogo(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(pedido): Json<PedidoArtigo>,
) -> impl IntoResponse {
    let _sess = exige_admin!(headers, state);
    if pedido.nome.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"erro":"Indica o nome do produto."})),
        )
            .into_response();
    }
    if pedido.nome.trim().as_bytes().len() > TAM_NOME {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"erro": format!("Nome: máximo {TAM_NOME} bytes.")})),
        )
            .into_response();
    }
    if pedido.imag.trim().as_bytes().len() > TAM_IMAG {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"erro": format!("Imagem: máximo {TAM_IMAG} caracteres.")})),
        )
            .into_response();
    }
    let a = pedido.para_base();
    let mut reg = ArtigoReg::from_artigo(&a);
    let bd = bd!(state, "artigo");
    match bd_gravar(bd.clone(), reg).await {
        Ok(n) => {
            if a.imag.is_empty() {
                let imag = nome_ficheiro_imag(n);
                reg.imag = lista_comum::str_para_arr(&imag);
                let _ = bd_alterar(bd, reg, n).await;
            }
            (StatusCode::CREATED, Json(reg.to_artigo(n))).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"erro": e.to_string()})),
        )
            .into_response(),
    }
}

/// PUT /api/catalogo/{id} — editar produto base (admin).
pub async fn editar_catalogo(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
    Json(pedido): Json<PedidoArtigo>,
) -> impl IntoResponse {
    let _sess = exige_admin!(headers, state);
    let bd = bd!(state, "artigo");
    let Some(mut reg) = bd_ler_dados::<ArtigoReg>(bd.clone(), id)
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
    if reg.utilizador != UTILIZADOR_BASE {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"erro":"Só produtos do catálogo base."})),
        )
            .into_response();
    }
    if pedido.nome.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"erro":"Indica o nome do produto."})),
        )
            .into_response();
    }
    if pedido.imag.trim().as_bytes().len() > TAM_IMAG {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"erro": format!("Imagem: máximo {TAM_IMAG} caracteres.")})),
        )
            .into_response();
    }
    let a = pedido.para_base();
    reg.nome = lista_comum::str_para_arr(&a.nome);
    reg.imag = lista_comum::str_para_arr(&a.imag);
    reg.unidade = a.unidade;
    reg.secao = a.secao;
    match bd_alterar(bd, reg, id).await {
        Ok(_) => (StatusCode::OK, Json(reg.to_artigo(id))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"erro": e.to_string()})),
        )
            .into_response(),
    }
}

/// DELETE /api/catalogo/{id} — apagar produto base (admin).
pub async fn remover_catalogo(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    let _sess = exige_admin!(headers, state);
    let bd = bd!(state, "artigo");
    let Some(reg) = bd_ler_dados::<ArtigoReg>(bd.clone(), id)
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
    if reg.utilizador != UTILIZADOR_BASE {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"erro":"Só produtos do catálogo base."})),
        )
            .into_response();
    }
    match bd_remover(bd, id).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"erro": e.to_string()})),
        )
            .into_response(),
    }
}

fn dono_pessoal(labnetcol_id: u64) -> Result<u64, (StatusCode, Json<serde_json::Value>)> {
    if labnetcol_id == 0 || labnetcol_id == UTILIZADOR_BASE {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "erro": "O catálogo pessoal é para utilizadores (n_reg ≠ 1). O Admin gere o catálogo base."
            })),
        ));
    }
    Ok(labnetcol_id)
}

fn validar_pedido(pedido: &PedidoArtigo) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if pedido.nome.trim().is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"erro":"Indica o nome do produto."})),
        ));
    }
    if pedido.nome.trim().as_bytes().len() > TAM_NOME {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"erro": format!("Nome: máximo {TAM_NOME} bytes.")})),
        ));
    }
    if pedido.imag.trim().as_bytes().len() > TAM_IMAG {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"erro": format!("Imagem: máximo {TAM_IMAG} caracteres.")})),
        ));
    }
    Ok(())
}

/// GET /api/meu-catalogo — produtos pessoais do utilizador autenticado.
pub async fn listar_meu_catalogo(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> impl IntoResponse {
    let sess = exige!(headers, state);
    let dono = match dono_pessoal(sess.labnetcol_id) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };
    let bd = bd!(state, "artigo");
    let mut lista: Vec<Artigo> = bd_listar::<ArtigoReg>(bd)
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, r)| r.utilizador == dono)
        .map(|(n, r)| r.to_artigo(n))
        .collect();
    lista.sort_by(|a, b| {
        a.secao
            .cmp(&b.secao)
            .then_with(|| a.nome.to_lowercase().cmp(&b.nome.to_lowercase()))
    });
    (StatusCode::OK, Json(lista)).into_response()
}

/// POST /api/meu-catalogo — criar produto pessoal.
pub async fn criar_meu_catalogo(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(pedido): Json<PedidoArtigo>,
) -> impl IntoResponse {
    let sess = exige!(headers, state);
    let dono = match dono_pessoal(sess.labnetcol_id) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };
    if let Err(e) = validar_pedido(&pedido) {
        return e.into_response();
    }
    let a = pedido.para_pessoal(dono);
    let mut reg = ArtigoReg::from_artigo(&a);
    let bd = bd!(state, "artigo");
    match bd_gravar(bd.clone(), reg).await {
        Ok(n) => {
            if a.imag.is_empty() {
                let imag = nome_ficheiro_imag(n);
                reg.imag = lista_comum::str_para_arr(&imag);
                let _ = bd_alterar(bd, reg, n).await;
            }
            (StatusCode::CREATED, Json(reg.to_artigo(n))).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"erro": e.to_string()})),
        )
            .into_response(),
    }
}

/// PUT /api/meu-catalogo/{id}
pub async fn editar_meu_catalogo(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
    Json(pedido): Json<PedidoArtigo>,
) -> impl IntoResponse {
    let sess = exige!(headers, state);
    let dono = match dono_pessoal(sess.labnetcol_id) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };
    if let Err(e) = validar_pedido(&pedido) {
        return e.into_response();
    }
    let bd = bd!(state, "artigo");
    let Some(mut reg) = bd_ler_dados::<ArtigoReg>(bd.clone(), id)
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
    if reg.utilizador != dono {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"erro":"Só podes editar os teus produtos."})),
        )
            .into_response();
    }
    let a = pedido.para_pessoal(dono);
    reg.nome = lista_comum::str_para_arr(&a.nome);
    reg.imag = lista_comum::str_para_arr(&a.imag);
    reg.unidade = a.unidade;
    reg.secao = a.secao;
    match bd_alterar(bd, reg, id).await {
        Ok(_) => (StatusCode::OK, Json(reg.to_artigo(id))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"erro": e.to_string()})),
        )
            .into_response(),
    }
}

/// DELETE /api/meu-catalogo/{id}
pub async fn remover_meu_catalogo(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> impl IntoResponse {
    let sess = exige!(headers, state);
    let dono = match dono_pessoal(sess.labnetcol_id) {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };
    let bd = bd!(state, "artigo");
    let Some(reg) = bd_ler_dados::<ArtigoReg>(bd.clone(), id)
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
    if reg.utilizador != dono {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({"erro":"Só podes apagar os teus produtos."})),
        )
            .into_response();
    }
    match bd_remover(bd, id).await {
        Ok(()) => (StatusCode::OK, Json(serde_json::json!({"ok": true}))).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"erro": e.to_string()})),
        )
            .into_response(),
    }
}

#[derive(Deserialize)]
pub struct PedidoImagem {
    pub dados_b64: String,
    #[serde(default)]
    pub extensao: String,
    /// Se indicado (ex. `imag12.png`), grava/substitui esse ficheiro e devolve o mesmo nome.
    #[serde(default)]
    pub nome_ficheiro: String,
}

/// POST /api/imagem — upload (autenticado). Substituir por nome: só admin.
pub async fn guardar_imagem(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(pedido): Json<PedidoImagem>,
) -> impl IntoResponse {
    let sess = exige!(headers, state);
    let raw = pedido
        .dados_b64
        .split(',')
        .last()
        .unwrap_or(pedido.dados_b64.as_str());
    let bytes = match base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        raw.trim(),
    ) {
        Ok(b) => b,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"erro":"Imagem inválida."})),
            )
                .into_response()
        }
    };
    if bytes.len() > 2_000_000 {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"erro":"Imagem demasiado grande (máx. 2 MB)."})),
        )
            .into_response();
    }

    let dir = imgs_dir();
    let _ = std::fs::create_dir_all(&dir);

    let nome = pedido.nome_ficheiro.trim().to_string();
    let (ficheiro, imag_campo) = if !nome.is_empty() {
        if !sess.admin {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({
                    "erro":"Só o administrador pode substituir um ficheiro pelo nome. Faz upload sem nome fixo."
                })),
            )
                .into_response();
        }
        if nome.as_bytes().len() > TAM_IMAG {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"erro": format!("Nome ficheiro: máx. {TAM_IMAG} caracteres.")})),
            )
                .into_response();
        }
        if !nome
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-')
            || nome.contains("..")
            || nome.starts_with('.')
        {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"erro":"Nome de ficheiro inválido."})),
            )
                .into_response();
        }
        (nome.clone(), nome)
    } else {
        let ext = {
            let e = pedido.extensao.trim().trim_start_matches('.').to_ascii_lowercase();
            match e.as_str() {
                "png" | "jpg" | "jpeg" | "webp" | "gif" => e,
                _ => "png".into(),
            }
        };
        let id = format!("{:08x}", agora_unix() as u32);
        let ficheiro = format!("i{id}.{ext}");
        if ficheiro.len() > TAM_IMAG {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"erro":"Id de imagem inválido."})),
            )
                .into_response();
        }
        (ficheiro.clone(), ficheiro)
    };

    let path = dir.join(&ficheiro);
    if let Err(e) = std::fs::write(&path, &bytes) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"erro": e.to_string()})),
        )
            .into_response();
    }
    (
        StatusCode::OK,
        Json(serde_json::json!({ "imag": imag_campo })),
    )
        .into_response()
}
