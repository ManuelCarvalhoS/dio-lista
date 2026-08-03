//! Tarefas agendadas no server (sempre ligado) — estilo cron interno.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use lista_comum::{
    agora_unix, moda_centimos, preco_unitario, ArtigoReg, CompraReg, RefAlteracao, RefAlteracaoReg,
    N_PRECOS_REF_GLOBAL,
};
use mcs_bd2::{bd_alterar, bd_gravar, bd_listar};

use crate::estado::AppState;

fn marker_path(data_dir: &str) -> PathBuf {
    PathBuf::from(data_dir).join(".job_ref_ultimo_dia")
}

fn dia_unix(secs: u64) -> u64 {
    secs / 86_400
}

fn ler_ultimo_dia(data_dir: &str) -> Option<u64> {
    let raw = std::fs::read_to_string(marker_path(data_dir)).ok()?;
    raw.trim().parse().ok()
}

fn gravar_dia(data_dir: &str, dia: u64) {
    let _ = std::fs::write(marker_path(data_dir), format!("{dia}\n"));
}

/// Lojas (até 3) onde o preço unitário `moda` apareceu nas compras recentes do artigo.
fn lojas_com_preco(compras: &[(u64, u32, u64)], moda: u32) -> Vec<u64> {
    let mut por_loja: HashMap<u64, u64> = HashMap::new();
    for &(loja_id, unit, ts) in compras {
        if loja_id == 0 || unit != moda {
            continue;
        }
        let e = por_loja.entry(loja_id).or_insert(0);
        if ts >= *e {
            *e = ts;
        }
    }
    let mut v: Vec<(u64, u64)> = por_loja.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    v.into_iter().take(3).map(|(id, _)| id).collect()
}

/// Recalcula `preco_referencia` de todos os artigos com compras (moda global).
pub async fn recalcular_precos_referencia(state: &AppState) -> (usize, usize) {
    let Some(bd_c) = state.bd.get("compra") else {
        return (0, 0);
    };
    let Some(bd_a) = state.bd.get("artigo") else {
        return (0, 0);
    };
    let bd_ref = state.bd.get("ref_alt").cloned();

    // artigo_id -> Vec<(loja_id, unit, comprado_em)>
    let mut por_artigo: HashMap<u64, Vec<(u64, u32, u64)>> = HashMap::new();
    for (_, r) in bd_listar::<CompraReg>(bd_c.clone())
        .await
        .unwrap_or_default()
    {
        if r.preco_cent == 0 {
            continue;
        }
        let unit = preco_unitario(r.preco_cent, r.qtd);
        if unit == 0 {
            continue;
        }
        por_artigo
            .entry(r.artigo_id)
            .or_default()
            .push((r.loja_id, unit, r.comprado_em));
    }

    let artigos: HashMap<u64, ArtigoReg> = bd_listar::<ArtigoReg>(bd_a.clone())
        .await
        .unwrap_or_default()
        .into_iter()
        .collect();

    let mut n_visto = 0usize;
    let mut n_alt = 0usize;

    for (artigo_id, mut amostras) in por_artigo {
        n_visto += 1;
        amostras.sort_by_key(|(_, _, ts)| *ts);
        let recent: Vec<(u64, u32, u64)> = amostras
            .iter()
            .rev()
            .take(N_PRECOS_REF_GLOBAL)
            .copied()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let units: Vec<u32> = recent.iter().map(|(_, u, _)| *u).collect();
        let Some(moda) = moda_centimos(&units) else {
            continue;
        };
        let Some(mut art) = artigos.get(&artigo_id).copied() else {
            continue;
        };
        let antes = art.preco_referencia;
        if antes == moda {
            continue;
        }
        art.preco_referencia = moda;
        if let Err(e) = bd_alterar(bd_a.clone(), art, artigo_id).await {
            log::warn!("job ref: artigo {artigo_id}: {e}");
            continue;
        }
        n_alt += 1;
        let lojas = lojas_com_preco(&recent, moda);
        if let Some(bd_r) = bd_ref.clone() {
            let alt = RefAlteracao::nova(artigo_id, antes, moda, &lojas);
            if let Err(e) = bd_gravar(bd_r, RefAlteracaoReg::from_alt(&alt)).await {
                log::warn!("job ref: log alteracao {artigo_id}: {e}");
            }
        }
    }

    (n_visto, n_alt)
}

/// Loop: verifica 1×/hora se já correu hoje; se não, recalcula.
pub fn spawn_job_precos_diario(state: AppState, data_dir: String) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(15)).await;
        loop {
            let agora = agora_unix();
            let hoje = dia_unix(agora);
            let ultimo = ler_ultimo_dia(&data_dir).unwrap_or(0);
            if hoje > ultimo {
                log::info!("Job preços de referência: a recalcular (dia {hoje})…");
                let (vistos, alts) = recalcular_precos_referencia(&state).await;
                gravar_dia(&data_dir, hoje);
                log::info!(
                    "Job preços: {vistos} artigos com compras, {alts} referências actualizadas"
                );
            }
            tokio::time::sleep(Duration::from_secs(3600)).await;
        }
    });
}
