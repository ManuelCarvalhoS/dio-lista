//! Import do catálogo base a partir de JSON (`seed/catalogo_base.json`).

use lista_comum::{
    nome_ficheiro_imag, str_para_arr, ArtigoReg, LojaReg, PedidoArtigo, LOJAS_SEED, TAM_NOME,
    UTILIZADOR_BASE,
};
use mcs_bd2::{bd_alterar, bd_gravar, bd_listar, EntityFiles};
use serde::Deserialize;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug, Deserialize)]
struct SeedItem {
    nome: String,
    #[serde(default)]
    unidade: u8,
    #[serde(default)]
    secao: u8,
    #[serde(default)]
    #[allow(dead_code)]
    imag: String, // no JSON para referência; gravamos sempre imag{n_reg}.png
}

pub async fn importar_catalogo_json(
    bd: Arc<Mutex<EntityFiles>>,
    path: &Path,
) -> anyhow::Result<usize> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("ler {}: {e}", path.display()))?;
    let itens: Vec<SeedItem> = serde_json::from_str(&raw)
        .map_err(|e| anyhow::anyhow!("JSON {}: {e}", path.display()))?;

    let existentes = bd_listar::<ArtigoReg>(bd.clone())
        .await
        .unwrap_or_default()
        .into_iter()
        .filter(|(_, r)| r.utilizador == UTILIZADOR_BASE)
        .count();
    if existentes > 0 {
        log::info!(
            "Seed: catálogo já tem {existentes} produtos — não importa {}",
            path.display()
        );
        return Ok(0);
    }

    let mut n = 0usize;
    for item in itens {
        let nome = item.nome.trim();
        if nome.is_empty() {
            continue;
        }
        if nome.as_bytes().len() > TAM_NOME {
            log::warn!("Seed: nome demasiado longo (bytes), ignorado: {nome}");
            continue;
        }
        let pedido = PedidoArtigo {
            nome: nome.to_string(),
            imag: String::new(),
            unidade: item.unidade,
            secao: item.secao,
        };
        let a = pedido.para_base();
        let reg = ArtigoReg::from_artigo(&a);
        let id = bd_gravar(bd.clone(), reg).await?;
        let imag = nome_ficheiro_imag(id);
        let mut reg2 = reg;
        reg2.imag = str_para_arr(&imag);
        bd_alterar(bd.clone(), reg2, id).await?;
        n += 1;
    }
    log::info!("Seed: importados {n} produtos de {}", path.display());
    Ok(n)
}

/// Garante lojas PT base (só acrescenta as que faltam pelo nome, utilizador = BASE).
pub async fn garantir_lojas(bd: Arc<Mutex<EntityFiles>>) -> anyhow::Result<usize> {
    let existentes = bd_listar::<LojaReg>(bd.clone()).await.unwrap_or_default();
    let nomes: std::collections::HashSet<String> = existentes
        .iter()
        .filter(|(_, r)| r.utilizador == UTILIZADOR_BASE)
        .map(|(id, r)| r.to_loja(*id).nome.to_lowercase())
        .filter(|n| !n.is_empty())
        .collect();
    let mut n = 0usize;
    for nome in LOJAS_SEED {
        if nomes.contains(&nome.to_lowercase()) {
            continue;
        }
        bd_gravar(bd.clone(), LojaReg::from_nome(UTILIZADOR_BASE, nome)).await?;
        n += 1;
    }
    Ok(n)
}
