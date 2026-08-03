use lista_comum::{
    Artigo, CompraHistorico, Ida, ItemListaDto, ListaCompra, Loja, PedidoArtigo, PedidoItem,
    PedidoItemPatch, PedidoLista, PedidoLoja, PedidoNovaIda, SessaoLista,
};
use reqwest::Client;
use serde::Deserialize;

pub struct ApiErro(pub String);

fn client() -> Client {
    Client::new()
}

/// Base absoluta para pedidos HTTP (reqwest no WASM rejeita URLs relativas).
fn api_base(api_url: &str) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        if let Some(origin) = web_sys::window()
            .and_then(|w| w.location().origin().ok())
            .filter(|s| !s.is_empty() && s != "null")
        {
            return origin;
        }
    }
    api_url.trim_end_matches('/').to_string()
}

fn api_join(api_url: &str, path: &str) -> String {
    let base = api_base(api_url);
    let path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    format!("{base}{path}")
}

async fn tratar(resp: reqwest::Response) -> String {
    let status = resp.status().as_u16();
    resp.json::<serde_json::Value>()
        .await
        .ok()
        .and_then(|v| v.get("erro").and_then(|e| e.as_str()).map(String::from))
        .unwrap_or_else(|| format!("Erro {status}"))
}

pub async fn sso_labnetcol(api_url: &str, token_jwt: &str) -> Result<SessaoLista, ApiErro> {
    let resp = client()
        .post(format!("{api_url}/api/sso/labnetcol"))
        .json(&serde_json::json!({ "token": token_jwt }))
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    resp.json().await.map_err(|e| ApiErro(e.to_string()))
}

pub async fn listar_catalogo(api_url: &str, token: &str) -> Result<Vec<Artigo>, ApiErro> {
    let resp = client()
        .get(format!("{api_url}/api/catalogo"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    resp.json().await.map_err(|e| ApiErro(e.to_string()))
}

pub async fn criar_catalogo(
    api_url: &str,
    token: &str,
    pedido: &PedidoArtigo,
) -> Result<Artigo, ApiErro> {
    let resp = client()
        .post(format!("{api_url}/api/catalogo"))
        .bearer_auth(token)
        .json(pedido)
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    resp.json().await.map_err(|e| ApiErro(e.to_string()))
}

pub async fn editar_catalogo(
    api_url: &str,
    token: &str,
    id: u64,
    pedido: &PedidoArtigo,
) -> Result<Artigo, ApiErro> {
    let resp = client()
        .put(format!("{api_url}/api/catalogo/{id}"))
        .bearer_auth(token)
        .json(pedido)
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    resp.json().await.map_err(|e| ApiErro(e.to_string()))
}

pub async fn remover_catalogo(api_url: &str, token: &str, id: u64) -> Result<(), ApiErro> {
    let resp = client()
        .delete(format!("{api_url}/api/catalogo/{id}"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    Ok(())
}

pub async fn listar_meu_catalogo(api_url: &str, token: &str) -> Result<Vec<Artigo>, ApiErro> {
    let resp = client()
        .get(format!("{api_url}/api/meu-catalogo"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    resp.json().await.map_err(|e| ApiErro(e.to_string()))
}

pub async fn criar_meu_catalogo(
    api_url: &str,
    token: &str,
    pedido: &PedidoArtigo,
) -> Result<Artigo, ApiErro> {
    let resp = client()
        .post(format!("{api_url}/api/meu-catalogo"))
        .bearer_auth(token)
        .json(pedido)
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    resp.json().await.map_err(|e| ApiErro(e.to_string()))
}

pub async fn editar_meu_catalogo(
    api_url: &str,
    token: &str,
    id: u64,
    pedido: &PedidoArtigo,
) -> Result<Artigo, ApiErro> {
    let resp = client()
        .put(format!("{api_url}/api/meu-catalogo/{id}"))
        .bearer_auth(token)
        .json(pedido)
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    resp.json().await.map_err(|e| ApiErro(e.to_string()))
}

pub async fn remover_meu_catalogo(api_url: &str, token: &str, id: u64) -> Result<(), ApiErro> {
    let resp = client()
        .delete(format!("{api_url}/api/meu-catalogo/{id}"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    Ok(())
}

#[derive(Deserialize, Clone, Debug)]
pub struct RespostaImagem {
    pub imag: String,
}

pub async fn upload_imagem(
    api_url: &str,
    token: &str,
    dados_b64: &str,
    extensao: &str,
    nome_ficheiro: Option<&str>,
) -> Result<RespostaImagem, ApiErro> {
    let mut body = serde_json::json!({
        "dados_b64": dados_b64,
        "extensao": extensao,
    });
    if let Some(n) = nome_ficheiro {
        body["nome_ficheiro"] = serde_json::json!(n);
    }
    let resp = client()
        .post(format!("{api_url}/api/imagem"))
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    resp.json().await.map_err(|e| ApiErro(e.to_string()))
}

#[derive(Deserialize, Clone, Debug)]
pub struct ModoDto {
    pub dev_login: bool,
    #[serde(default)]
    pub labnetcol_url: String,
    #[serde(default)]
    pub labnetcol_api: String,
}

pub async fn api_modo(api_url: &str) -> Result<ModoDto, ApiErro> {
    let resp = client()
        .get(format!("{api_url}/api/auth/modo"))
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    resp.json().await.map_err(|e| ApiErro(e.to_string()))
}

pub async fn listar_lojas(api_url: &str, token: &str) -> Result<Vec<Loja>, ApiErro> {
    let resp = client()
        .get(api_join(api_url, "/api/lojas"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    resp.json().await.map_err(|e| ApiErro(format!("Lojas JSON: {e}")))
}

pub async fn historico_precos_artigo(
    api_url: &str,
    token: &str,
    artigo_id: u64,
) -> Result<Vec<CompraHistorico>, ApiErro> {
    let resp = client()
        .get(api_join(
            api_url,
            &format!("/api/artigos/{artigo_id}/historico-precos"),
        ))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    resp.json().await.map_err(|e| ApiErro(e.to_string()))
}

pub async fn obter_ida(api_url: &str, token: &str, lista_id: u64) -> Result<Option<Ida>, ApiErro> {
    let resp = client()
        .get(format!("{api_url}/api/listas/{lista_id}/ida"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if resp.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    resp.json().await.map(Some).map_err(|e| ApiErro(e.to_string()))
}

pub async fn nova_ida(
    api_url: &str,
    token: &str,
    lista_id: u64,
    loja_id: u64,
) -> Result<Ida, ApiErro> {
    let resp = client()
        .post(format!("{api_url}/api/listas/{lista_id}/ida"))
        .bearer_auth(token)
        .json(&PedidoNovaIda { loja_id })
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    resp.json().await.map_err(|e| ApiErro(e.to_string()))
}

pub async fn criar_loja(api_url: &str, token: &str, nome: &str) -> Result<Loja, ApiErro> {
    let resp = client()
        .post(api_join(api_url, "/api/lojas"))
        .bearer_auth(token)
        .json(&PedidoLoja {
            nome: nome.to_string(),
        })
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    resp.json().await.map_err(|e| ApiErro(e.to_string()))
}

pub async fn listar_listas(api_url: &str, token: &str) -> Result<Vec<ListaCompra>, ApiErro> {
    let resp = client()
        .get(format!("{api_url}/api/listas"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    resp.json().await.map_err(|e| ApiErro(e.to_string()))
}

pub async fn criar_lista(
    api_url: &str,
    token: &str,
    nome: &str,
) -> Result<ListaCompra, ApiErro> {
    let resp = client()
        .post(format!("{api_url}/api/listas"))
        .bearer_auth(token)
        .json(&PedidoLista {
            nome: nome.to_string(),
        })
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    resp.json().await.map_err(|e| ApiErro(e.to_string()))
}

pub async fn remover_lista(api_url: &str, token: &str, id: u64) -> Result<(), ApiErro> {
    let resp = client()
        .delete(format!("{api_url}/api/listas/{id}"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    Ok(())
}

#[derive(Clone, Debug, Deserialize)]
pub struct TotaisLista {
    pub n_produtos: usize,
    pub n_comprados: usize,
    pub total_cent: u32,
    pub n_com_preco: usize,
    #[serde(default)]
    pub estimado_cent: u32,
    #[serde(default)]
    pub n_estimado: usize,
    #[serde(default)]
    pub estimado_lista_cent: u32,
    #[serde(default)]
    pub n_estimado_lista: usize,
}

pub async fn totais_lista(
    api_url: &str,
    token: &str,
    lista_id: u64,
) -> Result<TotaisLista, ApiErro> {
    let resp = client()
        .get(format!("{api_url}/api/listas/{lista_id}/totais"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    resp.json().await.map_err(|e| ApiErro(e.to_string()))
}

pub async fn listar_itens(
    api_url: &str,
    token: &str,
    lista_id: u64,
) -> Result<Vec<ItemListaDto>, ApiErro> {
    let resp = client()
        .get(format!("{api_url}/api/listas/{lista_id}/itens"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    resp.json().await.map_err(|e| ApiErro(e.to_string()))
}

pub async fn adicionar_item(
    api_url: &str,
    token: &str,
    lista_id: u64,
    artigo_id: u64,
    qtd: u16,
) -> Result<ItemListaDto, ApiErro> {
    let resp = client()
        .post(format!("{api_url}/api/listas/{lista_id}/itens"))
        .bearer_auth(token)
        .json(&PedidoItem { artigo_id, qtd })
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    resp.json().await.map_err(|e| ApiErro(e.to_string()))
}

pub async fn editar_item(
    api_url: &str,
    token: &str,
    id: u64,
    patch: &PedidoItemPatch,
) -> Result<ItemListaDto, ApiErro> {
    let resp = client()
        .put(format!("{api_url}/api/itens/{id}"))
        .bearer_auth(token)
        .json(patch)
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    resp.json().await.map_err(|e| ApiErro(e.to_string()))
}

pub async fn remover_item(api_url: &str, token: &str, id: u64) -> Result<(), ApiErro> {
    let resp = client()
        .delete(format!("{api_url}/api/itens/{id}"))
        .bearer_auth(token)
        .send()
        .await
        .map_err(|e| ApiErro(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(ApiErro(tratar(resp).await));
    }
    Ok(())
}
