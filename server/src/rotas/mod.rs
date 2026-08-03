use axum::routing::{delete, get, post, put};
use axum::Router;

use crate::estado::AppState;

pub mod artigo_rota;
pub mod auth_rota;
pub mod lista_rota;
pub mod loja_rota;

pub fn api_router(state: AppState) -> Router {
    Router::new()
        .route("/sso/labnetcol", post(auth_rota::sso_labnetcol))
        .route("/auth/modo", get(auth_rota::modo))
        .route("/auth/dev", post(auth_rota::login_dev))
        .route("/eu", get(auth_rota::eu))
        .route("/catalogo", get(artigo_rota::listar_catalogo))
        .route("/catalogo", post(artigo_rota::criar_catalogo))
        .route("/catalogo/{id}", put(artigo_rota::editar_catalogo))
        .route("/catalogo/{id}", delete(artigo_rota::remover_catalogo))
        .route("/meu-catalogo", get(artigo_rota::listar_meu_catalogo))
        .route("/meu-catalogo", post(artigo_rota::criar_meu_catalogo))
        .route("/meu-catalogo/{id}", put(artigo_rota::editar_meu_catalogo))
        .route("/meu-catalogo/{id}", delete(artigo_rota::remover_meu_catalogo))
        .route("/imagem", post(artigo_rota::guardar_imagem))
        .route("/lojas", get(loja_rota::listar_lojas))
        .route("/lojas", post(loja_rota::criar_loja))
        .route("/listas", get(lista_rota::listar_listas))
        .route("/listas", post(lista_rota::criar_lista))
        .route("/listas/{id}", delete(lista_rota::remover_lista))
        .route("/listas/{id}/itens", get(lista_rota::listar_itens))
        .route("/listas/{id}/itens", post(lista_rota::adicionar_item))
        .route("/listas/{id}/totais", get(lista_rota::totais_lista))
        .route("/listas/{id}/ida", get(lista_rota::obter_ida))
        .route("/listas/{id}/ida", post(lista_rota::nova_ida))
        .route("/itens/{id}", put(lista_rota::editar_item))
        .route("/itens/{id}", delete(lista_rota::remover_item))
        .route(
            "/artigos/{id}/historico-precos",
            get(lista_rota::historico_precos_artigo),
        )
        .route(
            "/ref-alteracoes",
            get(lista_rota::listar_ref_alteracoes),
        )
        .with_state(state)
}
