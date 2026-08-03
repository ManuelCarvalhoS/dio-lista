//! Lojas — listar (base + minhas) e criar (pessoal).

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use lista_comum::{Loja, LojaReg, PedidoLoja, TAM_NOME, UTILIZADOR_BASE};
use mcs_bd2::{bd_gravar, bd_listar};

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

/// GET /api/lojas — seed PT (base) + lojas do utilizador.
pub async fn listar_lojas(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let sess = exige!(headers, state);
    let dono = sess.labnetcol_id;
    let bd = bd!(state, "loja");
    match bd_listar::<LojaReg>(bd).await {
        Ok(regs) => {
            let mut out: Vec<Loja> = regs
                .into_iter()
                .map(|(id, r)| r.to_loja(id))
                .filter(|l| {
                    !l.nome.is_empty()
                        && (l.utilizador == UTILIZADOR_BASE || l.utilizador == dono)
                })
                .collect();
            out.sort_by(|a, b| {
                // Base primeiro, depois pessoais; dentro: nome.
                let oa = if a.utilizador == UTILIZADOR_BASE { 0 } else { 1 };
                let ob = if b.utilizador == UTILIZADOR_BASE { 0 } else { 1 };
                oa.cmp(&ob)
                    .then_with(|| a.nome.to_lowercase().cmp(&b.nome.to_lowercase()))
            });
            (StatusCode::OK, Json(out)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"erro": e.to_string()})),
        )
            .into_response(),
    }
}

/// POST /api/lojas — cria loja pessoal (ou devolve existente base/minha com o mesmo nome).
pub async fn criar_loja(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(pedido): Json<PedidoLoja>,
) -> impl IntoResponse {
    let sess = exige!(headers, state);
    let dono = sess.labnetcol_id;
    let nome = pedido.nome.trim().to_string();
    if nome.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"erro":"Indica o nome da loja."})),
        )
            .into_response();
    }
    if nome.as_bytes().len() > TAM_NOME {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"erro": format!("Máximo {TAM_NOME} bytes.")})),
        )
            .into_response();
    }
    let bd = bd!(state, "loja");
    if let Ok(regs) = bd_listar::<LojaReg>(bd.clone()).await {
        let mut minha = None;
        for (id, r) in regs {
            let l = r.to_loja(id);
            if l.nome.is_empty() || !l.nome.eq_ignore_ascii_case(&nome) {
                continue;
            }
            if l.utilizador == UTILIZADOR_BASE {
                return (StatusCode::OK, Json(l)).into_response();
            }
            if l.utilizador == dono {
                minha = Some(l);
            }
        }
        if let Some(l) = minha {
            return (StatusCode::OK, Json(l)).into_response();
        }
    }
    match bd_gravar(bd, LojaReg::from_nome(dono, &nome)).await {
        Ok(id) => {
            let loja = Loja {
                n_reg: id,
                utilizador: dono,
                nome,
            };
            (StatusCode::CREATED, Json(loja)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"erro": e.to_string()})),
        )
            .into_response(),
    }
}
