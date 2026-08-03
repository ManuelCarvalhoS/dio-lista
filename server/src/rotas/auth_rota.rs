use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use lista_comum::{SessaoLista, UTILIZADOR_BASE};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::estado::AppState;

#[derive(Debug, Serialize, Deserialize)]
struct SsoClaims {
    sub: u64,
    #[serde(default)]
    pseudonimo: String,
    #[serde(default)]
    contacto: String,
    #[serde(default)]
    app: String,
    #[serde(default)]
    tipo: u8,
    exp: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct ListaClaims {
    sub: u64,
    nome: String,
    #[serde(default)]
    tipo: u8,
    #[serde(default)]
    admin: bool,
    exp: u64,
}

#[derive(Deserialize)]
pub struct PedidoSso {
    pub token: String,
}

pub struct InfoSessao {
    pub labnetcol_id: u64,
    pub nome: String,
    pub tipo: u8,
    pub admin: bool,
}

pub fn verificar_token(headers: &HeaderMap, secret: &str) -> Option<InfoSessao> {
    let auth = headers.get("Authorization")?.to_str().ok()?;
    let token = auth.strip_prefix("Bearer ")?;
    let data = decode::<ListaClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )
    .ok()?;
    Some(InfoSessao {
        labnetcol_id: data.claims.sub,
        nome: data.claims.nome,
        tipo: data.claims.tipo,
        admin: data.claims.admin,
    })
}

fn emitir_sessao(
    state: &AppState,
    sub: u64,
    nome: String,
    tipo: u8,
) -> axum::response::Response {
    let admin = state.e_admin(sub, tipo);
    let exp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        + 7 * 24 * 3600;
    let claims = ListaClaims {
        sub,
        nome: nome.clone(),
        tipo,
        admin,
        exp,
    };
    match encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.jwt_secret.as_bytes()),
    ) {
        Ok(token) => (
            StatusCode::OK,
            Json(SessaoLista {
                token,
                labnetcol_id: sub,
                nome,
                admin,
            }),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"erro":"Erro ao criar sessão."})),
        )
            .into_response(),
    }
}

/// GET /api/auth/modo
pub async fn modo(State(state): State<AppState>) -> impl IntoResponse {
    Json(serde_json::json!({
        "dev_login": state.dev_login,
        "labnetcol_url": state.labnetcol_url,
        "labnetcol_api": state.labnetcol_api,
    }))
}

#[derive(Deserialize)]
pub struct PedidoDev {
    #[serde(default)]
    pub nome: String,
    #[serde(default)]
    pub id: u64,
    /// Se true (ou nome "admin"), sessão admin para testar o catálogo.
    #[serde(default)]
    pub admin: bool,
}

fn id_de_nome(nome: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in nome.trim().to_lowercase().bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    if h == 0 {
        1
    } else {
        h
    }
}

pub async fn login_dev(
    State(state): State<AppState>,
    Json(pedido): Json<PedidoDev>,
) -> impl IntoResponse {
    if !state.dev_login {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "erro": "Entrada directa desligada. Usa o LabNetCol."
            })),
        )
            .into_response();
    }
    let nome = {
        let n = pedido.nome.trim();
        if n.is_empty() {
            "Convidado".to_string()
        } else {
            n.to_string()
        }
    };
    let id = if pedido.id != 0 {
        pedido.id
    } else if pedido.admin || nome.eq_ignore_ascii_case("admin") {
        UTILIZADOR_BASE
    } else {
        let mut id = id_de_nome(&nome);
        // Reservado ao catálogo base / Admin típico
        if id == 0 || id == UTILIZADOR_BASE {
            id = 2;
        }
        id
    };
    let tipo = if pedido.admin || nome.eq_ignore_ascii_case("admin") || state.admin_ids.contains(&id)
    {
        1
    } else {
        0
    };
    emitir_sessao(&state, id, nome, tipo)
}

pub async fn sso_labnetcol(
    State(state): State<AppState>,
    Json(pedido): Json<PedidoSso>,
) -> impl IntoResponse {
    let data = match decode::<SsoClaims>(
        pedido.token.trim(),
        &DecodingKey::from_secret(state.sso_secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(d) => d,
        Err(e) => {
            log::warn!("SSO LabNetCol rejeitado: {e}");
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"erro":"Token SSO inválido ou expirado."})),
            )
                .into_response()
        }
    };

    let n_reg = data.claims.sub;
    let nome = {
        let p = data.claims.pseudonimo.trim();
        if p.is_empty() {
            format!("Utilizador {n_reg}")
        } else {
            p.to_string()
        }
    };
    emitir_sessao(&state, n_reg, nome, data.claims.tipo)
}

pub async fn eu(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    match verificar_token(&headers, &state.jwt_secret) {
        Some(s) => (
            StatusCode::OK,
            Json(serde_json::json!({
                "labnetcol_id": s.labnetcol_id,
                "nome": s.nome,
                "admin": s.admin,
            })),
        )
            .into_response(),
        None => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"erro":"Não autenticado"})),
        )
            .into_response(),
    }
}
