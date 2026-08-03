use axum::{http::Method, Router};
use mcs_bd2::{abrir_ficheiros, CampoIndice};
use std::collections::HashSet;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

use lista_comum::{
    TAM_REG_ARTIGO_U64, TAM_REG_COMPRA_U64, TAM_REG_IDA_U64, TAM_REG_ITEM_U64, TAM_REG_LISTA_U64,
    TAM_REG_LOJA_U64, TAM_REG_REF_ALT_U64,
};

mod estado;
mod jobs;
mod rotas;
mod seed;

static INDICES_VAZIOS: &[CampoIndice] = &[];

fn entities() -> Vec<(String, u32, Vec<String>, u64, &'static [CampoIndice])> {
    vec![
        (
            "artigo".into(),
            1,
            vec!["artigo.dad".into(), "artigo.h1".into(), "artigo.h2".into()],
            TAM_REG_ARTIGO_U64,
            INDICES_VAZIOS,
        ),
        (
            "lista".into(),
            2,
            vec!["lista.dad".into(), "lista.h1".into(), "lista.h2".into()],
            TAM_REG_LISTA_U64,
            INDICES_VAZIOS,
        ),
        (
            "item".into(),
            3,
            vec!["item.dad".into(), "item.h1".into(), "item.h2".into()],
            TAM_REG_ITEM_U64,
            INDICES_VAZIOS,
        ),
        (
            "loja".into(),
            4,
            vec!["loja.dad".into(), "loja.h1".into(), "loja.h2".into()],
            TAM_REG_LOJA_U64,
            INDICES_VAZIOS,
        ),
        (
            "compra".into(),
            5,
            vec!["compra.dad".into(), "compra.h1".into(), "compra.h2".into()],
            TAM_REG_COMPRA_U64,
            INDICES_VAZIOS,
        ),
        (
            "ida".into(),
            6,
            vec!["ida.dad".into(), "ida.h1".into(), "ida.h2".into()],
            TAM_REG_IDA_U64,
            INDICES_VAZIOS,
        ),
        (
            "ref_alt".into(),
            7,
            vec![
                "ref_alt.dad".into(),
                "ref_alt.h1".into(),
                "ref_alt.h2".into(),
            ],
            TAM_REG_REF_ALT_U64,
            INDICES_VAZIOS,
        ),
    ]
}

fn parse_admin_ids() -> HashSet<u64> {
    let raw = std::env::var("LISTA_ADMIN_IDS").unwrap_or_else(|_| "1".into());
    raw.split(',')
        .filter_map(|s| s.trim().parse::<u64>().ok())
        .filter(|n| *n > 0)
        .collect()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    simple_logger::init_with_level(log::Level::Info).ok();

    let porta: u16 = std::env::var("LISTA_PORT")
        .unwrap_or_else(|_| "8088".into())
        .parse()
        .unwrap_or(8088);

    let data_dir = std::env::var("LISTA_DATA_DIR").unwrap_or_else(|_| "./data".into());
    std::fs::create_dir_all(format!("{data_dir}/imgs"))?;

    // Tabelas antigas já não usadas
    for nome in ["marca", "nota"] {
        for ext in ["dad", "h1", "h2"] {
            let _ = std::fs::remove_file(format!("{data_dir}/{nome}.{ext}"));
        }
    }
    // Apagar artigo só se LISTA_WIPE=1 (reinício limpo em testes)
    let wipe = matches!(
        std::env::var("LISTA_WIPE")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    );
    if wipe {
        log::warn!("LISTA_WIPE: a apagar catálogo artigo.*");
        for ext in ["dad", "h1", "h2"] {
            let _ = std::fs::remove_file(format!("{data_dir}/artigo.{ext}"));
        }
    }

    // Layout loja v2: utilizador @0 + nome @8. Marker evita reseed a cada boot.
    let loja_v2 = format!("{data_dir}/.loja_layout_v2");
    let force_loja = matches!(
        std::env::var("LISTA_RESEED_LOJAS")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    );
    if force_loja || !std::path::Path::new(&loja_v2).exists() {
        log::warn!("Lojas: a recriar ficheiros (layout com utilizador / seed base)");
        for ext in ["dad", "h1", "h2"] {
            let _ = std::fs::remove_file(format!("{data_dir}/loja.{ext}"));
        }
        let _ = std::fs::write(&loja_v2, b"2\n");
    }

    let dist = std::env::var("LISTA_DIST").unwrap_or_else(|_| "../frontend-web/dist".into());

    let bd = abrir_ficheiros(&data_dir, &data_dir, entities()).expect("Falha ao abrir BD Lista");
    log::info!("BD Lista (catálogo) aberta em {data_dir}");

    let seed = matches!(
        std::env::var("LISTA_SEED")
            .unwrap_or_default()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    );
    if seed {
        let seed_path = std::env::var("LISTA_SEED_FILE").unwrap_or_else(|_| {
            // default: ../seed/catalogo_base.json relativamente ao binário / cwd do server
            "../seed/catalogo_base.json".into()
        });
        let path = std::path::PathBuf::from(&seed_path);
        let path = if path.is_absolute() {
            path
        } else {
            std::env::current_dir()
                .unwrap_or_default()
                .join(&path)
        };
        match seed::importar_catalogo_json(bd.get("artigo").expect("artigo").clone(), &path).await
        {
            Ok(n) => log::info!("LISTA_SEED: {n} produtos"),
            Err(e) => log::error!("LISTA_SEED falhou: {e}"),
        }
    }

    match seed::garantir_lojas(bd.get("loja").expect("loja").clone()).await {
        Ok(n) if n > 0 => log::info!("Seed lojas: {n} novas"),
        Ok(_) => {}
        Err(e) => log::error!("Seed lojas falhou: {e}"),
    }

    let jwt_secret = std::env::var("LISTA_JWT_SECRET").unwrap_or_else(|_| "lista-jwt-dev".into());
    let dev_login = match std::env::var("LISTA_DEV_LOGIN") {
        Ok(v) => !matches!(v.to_ascii_lowercase().as_str(), "0" | "false" | "no" | "off"),
        Err(_) => true,
    };

    let admin_ids = parse_admin_ids();
    log::info!("Admin IDs LabNetCol: {admin_ids:?}");

    let state = estado::AppState {
        bd: Arc::new(bd),
        sso_secret: {
            let s = std::env::var("LABNETCOL_SECRET")
                .unwrap_or_else(|_| "labnetcol-sso-dev-secret".into());
            if s == "labnetcol-sso-dev-secret" {
                log::warn!("LABNETCOL_SECRET em falta/.env — a usar default de desenvolvimento");
            } else {
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut h = DefaultHasher::new();
                s.hash(&mut h);
                log::info!(
                    "LABNETCOL_SECRET carregado (fp={:08x}, len={})",
                    h.finish() as u32,
                    s.len()
                );
            }
            s
        },
        jwt_secret,
        labnetcol_url: std::env::var("LABNETCOL_FRONTEND_URL")
            .unwrap_or_else(|_| "http://localhost:8080".into()),
        labnetcol_api: std::env::var("LABNETCOL_API_URL")
            .unwrap_or_else(|_| "http://localhost:8080".into()),
        dev_login,
        admin_ids,
    };
    if state.dev_login {
        log::warn!("Entrada directa activa (LISTA_DEV_LOGIN)");
    }

    // Job diário: moda dos últimos 500 preços unitários (todos os users) → preco_referencia.
    jobs::spawn_job_precos_diario(state.clone(), data_dir.clone());

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers(Any);

    let imgs = format!("{data_dir}/imgs");
    let app = Router::new()
        .nest("/api", rotas::api_router(state))
        .nest_service("/imgs", ServeDir::new(&imgs))
        .fallback_service(
            ServeDir::new(&dist).fallback(ServeFile::new(format!("{dist}/index.html"))),
        )
        .layer(cors);

    let addr = format!("127.0.0.1:{porta}");
    log::info!("Lista a escutar em http://{addr}");
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
