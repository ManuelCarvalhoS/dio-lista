#![allow(non_snake_case)]

mod api;
mod sessao;

use api::*;
use dioxus::prelude::*;
use lista_comum::{
    Artigo, CompraHistorico, ItemListaDto, ListaCompra, Loja, PedidoArtigo, PedidoItemPatch, Secao,
    SessaoLista, Unidade,
};
use sessao::*;
use std::collections::HashMap;

const CSS: Asset = asset!("/assets/main.css");
const LABNETCOL_URL_LOCAL: &str = "http://localhost:8080";
const LABNETCOL_URL_PROD: &str = "https://labnetcol.pt";
const API_LOCAL: &str = "http://localhost:8088";

/// Portal LabNetCol: prod se a Lista corre em *.labnetcol.pt; senão localhost.
fn labnetcol_url_padrao() -> String {
    let host = web_sys::window()
        .and_then(|w| w.location().hostname().ok())
        .unwrap_or_default();
    if host.ends_with("labnetcol.pt") {
        LABNETCOL_URL_PROD.into()
    } else {
        LABNETCOL_URL_LOCAL.into()
    }
}

const COR_FUNDO: &str = "#f4f7fb";
const COR_SUBTEXTO: &str = "#666666";
const COR_ERRO: &str = "#c0392b";
const ESTILO_BTN: &str = "padding:10px 24px; background:#1a4a8c; border:none;
    border-radius:6px; color:white; font-size:0.95rem; cursor:pointer; font-weight:500;";

fn abrir_url(url: &str) {
    if let Some(w) = web_sys::window() {
        let _ = w.location().set_href(url);
    }
}

fn cent_para_euros(c: u32) -> String {
    format!("{},{:02} €", c / 100, c % 100)
}

fn cent_para_curto(c: u32) -> String {
    format!("{},{:02}", c / 100, c % 100)
}

/// Data curta a partir de unix (segundos) — DD/MM.
fn data_curta(unix: u64) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let d = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64((unix as f64) * 1000.0));
        let day = d.get_date();
        let month = d.get_month() + 1;
        return format!("{day:02}/{month:02}");
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = unix;
        "—".into()
    }
}

/// "1,99" / "1.99" / "2" → cêntimos; vazio → 0; inválido → None.
fn euros_para_cent(s: &str) -> Option<u32> {
    let t = s
        .trim()
        .replace('€', "")
        .replace(' ', "")
        .replace(',', ".");
    if t.is_empty() {
        return Some(0);
    }
    let v: f64 = t.parse().ok()?;
    if !(0.0..=1_000_000.0).contains(&v) {
        return None;
    }
    Some((v * 100.0).round() as u32)
}

fn encode_query(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn encode_b64(data: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let a = chunk[0] as u32;
        let b = chunk.get(1).copied().unwrap_or(0) as u32;
        let c = chunk.get(2).copied().unwrap_or(0) as u32;
        let n = (a << 16) | (b << 8) | c;
        out.push(T[((n >> 18) & 63) as usize] as char);
        out.push(T[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(T[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(T[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Entrada sempre pelo LabNetCol; regresso com SSO.
fn url_entrada_labnetcol(portal: &str) -> String {
    let origem = web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .filter(|s| !s.is_empty() && s != "null")
        .unwrap_or_else(|| API_LOCAL.into());
    let fallback = labnetcol_url_padrao();
    let base = if portal.trim().is_empty() {
        fallback.as_str()
    } else {
        portal.trim().trim_end_matches('/')
    };
    format!("{base}/?p=login&return_to={}", encode_query(&origem))
}

fn api_url_padrao() -> String {
    let Some(origin) = web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .filter(|s| !s.is_empty() && s != "null")
    else {
        return API_LOCAL.into();
    };
    // dx serve noutro porto → API no lista_serv
    if (origin.starts_with("http://localhost:") || origin.starts_with("http://127.0.0.1:"))
        && origin != API_LOCAL
        && origin != "http://127.0.0.1:8088"
    {
        return API_LOCAL.into();
    }
    origin
}

fn ler_token_sso_url() -> Option<String> {
    let win = web_sys::window()?;
    let loc = win.location();
    let _path = loc.pathname().ok()?;
    let search = loc.search().ok().unwrap_or_default();
    let q = search.trim_start_matches('?');
    for par in q.split('&') {
        let mut kv = par.splitn(2, '=');
        if let (Some("token"), Some(v)) = (kv.next(), kv.next()) {
            if !v.is_empty() {
                return Some(url_decode(v));
            }
        }
    }
    None
}

fn ler_erro_url() -> Option<String> {
    let win = web_sys::window()?;
    let search = win.location().search().ok().unwrap_or_default();
    let q = search.trim_start_matches('?');
    for par in q.split('&') {
        let mut kv = par.splitn(2, '=');
        if let (Some("erro"), Some(v)) = (kv.next(), kv.next()) {
            if !v.is_empty() {
                return Some(url_decode(v));
            }
        }
    }
    None
}

fn url_decode(s: &str) -> String {
    let mut out = Vec::new();
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' if i + 2 < b.len() => {
                let hex = |c: u8| match c {
                    b'0'..=b'9' => Some(c - b'0'),
                    b'a'..=b'f' => Some(c - b'a' + 10),
                    b'A'..=b'F' => Some(c - b'A' + 10),
                    _ => None,
                };
                if let (Some(hi), Some(lo)) = (hex(b[i + 1]), hex(b[i + 2])) {
                    out.push((hi << 4) | lo);
                    i += 3;
                } else {
                    out.push(b[i]);
                    i += 1;
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            c => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn limpar_url() {
    if let Some(win) = web_sys::window() {
        if let Ok(hist) = win.history() {
            let _ = hist.replace_state_with_url(&wasm_bindgen::JsValue::NULL, "", Some("/"));
        }
    }
}

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let config = ler_config();
    let api_padrao = if config.api_url.is_empty() {
        api_url_padrao()
    } else {
        config.api_url
    };
    let api_url = use_signal(move || api_padrao.clone());
    let mut sessao = use_signal(ler_sessao);
    let mut sso_erro = use_signal(|| ler_erro_url());
    let mut sso_a_processar = use_signal(|| false);

    // SSO à entrada (+ limpar ?erro= da URL)
    use_effect(move || {
        if ler_erro_url().is_some() {
            limpar_url();
        }
        if let Some(tok) = ler_token_sso_url() {
            sso_a_processar.set(true);
            let url = api_url();
            spawn(async move {
                match sso_labnetcol(&url, &tok).await {
                    Ok(s) => {
                        guardar_sessao(&s);
                        sessao.set(Some(s));
                        limpar_url();
                    }
                    Err(e) => sso_erro.set(Some(e.0)),
                }
                sso_a_processar.set(false);
            });
        }
    });

    rsx! {
        document::Stylesheet { href: CSS }
        document::Link {
            rel: "stylesheet",
            href: "https://fonts.googleapis.com/css2?family=Fraunces:opsz,wght@9..144,600;9..144,700&family=Source+Sans+3:wght@400;500;600&display=swap",
        }
        document::Title { "Lista — compras" }

        if sso_a_processar() {
            div {
                style: "display:flex; justify-content:center; align-items:center;
                        min-height:100vh; background:{COR_FUNDO};",
                div {
                    style: "background:white; padding:40px; border-radius:12px; width:360px;
                            box-shadow:0 2px 16px rgba(0,0,0,0.08);",
                    h1 { style: "margin:0 0 4px 0; font-size:1.8rem; color:#1a1a1a;", "Lista" }
                    p { style: "color:{COR_SUBTEXTO}; margin:0; font-size:0.9rem;",
                        "A validar sessão…"
                    }
                }
            }
        } else if let Some(e) = sso_erro() {
            div {
                style: "display:flex; justify-content:center; align-items:center;
                        min-height:100vh; background:{COR_FUNDO};",
                div {
                    style: "background:white; padding:40px; border-radius:12px; width:360px;
                            box-shadow:0 2px 16px rgba(0,0,0,0.08);",
                    h1 { style: "margin:0 0 4px 0; font-size:1.8rem; color:#1a1a1a;", "Lista" }
                    p { style: "color:{COR_ERRO}; font-size:0.9rem; margin:12px 0;", "{e}" }
                    button {
                        style: "{ESTILO_BTN}",
                        onclick: move |_| {
                            sso_erro.set(None);
                            limpar_url();
                            abrir_url(&url_entrada_labnetcol(&labnetcol_url_padrao()));
                        },
                        "Entrar no LabNetCol"
                    }
                }
            }
        } else if let Some(s) = sessao() {
            AppLogada { api_url, sessao: s, on_sair: move |_| {
                limpar_sessao();
                sessao.set(None);
                // Termina também a sessão do portal — senão o SSO reentra de imediato
                abrir_url(&format!("{}/?p=sair", labnetcol_url_padrao().trim_end_matches('/')));
            }}
        } else {
            PortaRedirectLabNetCol { api_url }
        }
    }
}

/// Sem cartão local: a identidade fica no LabNetCol.
#[component]
fn PortaRedirectLabNetCol(api_url: Signal<String>) -> Element {
    let mut portal = use_signal(labnetcol_url_padrao);
    let mut ja_foi = use_signal(|| false);

    use_effect(move || {
        let url = api_url();
        spawn(async move {
            let dest = match api_modo(&url).await {
                Ok(m) if !m.labnetcol_url.trim().is_empty() => {
                    m.labnetcol_url.trim_end_matches('/').to_string()
                }
                _ => labnetcol_url_padrao(),
            };
            portal.set(dest.clone());
            if !ja_foi() {
                ja_foi.set(true);
                abrir_url(&url_entrada_labnetcol(&dest));
            }
        });
    });

    rsx! {
        div {
            style: "display:flex; justify-content:center; align-items:center;
                    min-height:100vh; background:{COR_FUNDO};",
            div {
                style: "background:white; padding:40px; border-radius:12px; width:360px;
                        box-shadow:0 2px 16px rgba(0,0,0,0.08); text-align:center;",
                h1 { style: "margin:0 0 8px 0; font-size:1.8rem; color:#1a1a1a;", "Lista" }
                p { style: "color:{COR_SUBTEXTO}; margin:0 0 20px 0; font-size:0.95rem;",
                    "A abrir o LabNetCol para entrares…"
                }
                button {
                    style: "{ESTILO_BTN}",
                    onclick: move |_| abrir_url(&url_entrada_labnetcol(&portal())),
                    "Abrir LabNetCol"
                }
            }
        }
    }
}

#[component]
fn AppLogada(
    api_url: Signal<String>,
    sessao: SessaoLista,
    on_sair: EventHandler<()>,
) -> Element {
    if sessao.admin {
        rsx! { CatalogoAdmin { api_url, sessao, on_sair } }
    } else {
        rsx! { CatalogoUtilizador { api_url, sessao, on_sair } }
    }
}

/// Admin: listas / compras + gestão do catálogo base.
#[component]
fn CatalogoAdmin(
    api_url: Signal<String>,
    sessao: SessaoLista,
    on_sair: EventHandler<()>,
) -> Element {
    let mut aba = use_signal(|| 0u8); // 0 = listas, 1 = compras, 2 = catálogo
    rsx! {
        div { class: "app",
            header { class: "top",
                div {
                    h1 { "Lista" }
                    p { class: "sub", "Olá, {sessao.nome} — admin" }
                }
                button { class: "btn-ghost", onclick: move |_| on_sair.call(()), "Sair" }
            }
            div { class: "filtros-secao abas",
                button {
                    class: if aba() == 0 { "chip on" } else { "chip" },
                    onclick: move |_| aba.set(0),
                    "Listas"
                }
                button {
                    class: if aba() == 1 { "chip on" } else { "chip" },
                    onclick: move |_| aba.set(1),
                    "Compras"
                }
                button {
                    class: if aba() == 2 { "chip on" } else { "chip" },
                    onclick: move |_| aba.set(2),
                    "Catálogo base"
                }
            }
            if aba() == 0 {
                ComprasPainel { api_url, sessao: sessao.clone(), modo_ida: false }
            } else if aba() == 1 {
                ComprasPainel { api_url, sessao: sessao.clone(), modo_ida: true }
            } else {
                CatalogoPainel {
                    api_url,
                    sessao: sessao.clone(),
                    on_sair,
                    modo: ModoCat::BaseAdmin,
                    embedido: true,
                }
            }
        }
    }
}

/// Abas: listas + compras + pessoais + catálogo base.
#[component]
fn CatalogoUtilizador(
    api_url: Signal<String>,
    sessao: SessaoLista,
    on_sair: EventHandler<()>,
) -> Element {
    let mut aba = use_signal(|| 0u8); // 0 = listas, 1 = compras, 2 = meus, 3 = base
    let token = use_signal(|| sessao.token.clone());
    let mut base = use_signal(Vec::<Artigo>::new);
    let mut erro_base = use_signal(|| Option::<String>::None);
    let mut filtro_secao = use_signal(|| Option::<u8>::None);
    let mut base_carregado = use_signal(|| false);

    let carregar_base = move || {
        let url = api_url();
        let t = token();
        spawn(async move {
            match listar_catalogo(&url, &t).await {
                Ok(l) => {
                    base.set(l);
                    erro_base.set(None);
                    base_carregado.set(true);
                }
                Err(e) => {
                    base.set(Vec::new());
                    erro_base.set(Some(e.0));
                    base_carregado.set(true);
                }
            }
        });
    };

    use_effect(move || {
        carregar_base();
    });

    let filtrados_base = {
        let f = filtro_secao();
        base()
            .into_iter()
            .filter(|a| f.map(|s| a.secao == s).unwrap_or(true))
            .collect::<Vec<_>>()
    };

    rsx! {
        div { class: "app",
            header { class: "top",
                div {
                    h1 { "Lista" }
                    p { class: "sub", "Olá, {sessao.nome} — listas, compras e catálogos" }
                }
                button { class: "btn-ghost", onclick: move |_| on_sair.call(()), "Sair" }
            }
            div { class: "filtros-secao abas",
                button {
                    class: if aba() == 0 { "chip on" } else { "chip" },
                    onclick: move |_| aba.set(0),
                    "Listas"
                }
                button {
                    class: if aba() == 1 { "chip on" } else { "chip" },
                    onclick: move |_| aba.set(1),
                    "Compras"
                }
                button {
                    class: if aba() == 2 { "chip on" } else { "chip" },
                    onclick: move |_| aba.set(2),
                    "Os meus"
                }
                button {
                    class: if aba() == 3 { "chip on" } else { "chip" },
                    onclick: move |_| {
                        aba.set(3);
                        carregar_base();
                    },
                    "Catálogo base ({base().len()})"
                }
            }
            if aba() == 0 {
                ComprasPainel { api_url, sessao: sessao.clone(), modo_ida: false }
            } else if aba() == 1 {
                ComprasPainel { api_url, sessao: sessao.clone(), modo_ida: true }
            } else if aba() == 2 {
                CatalogoPainel {
                    api_url,
                    sessao: sessao.clone(),
                    on_sair,
                    modo: ModoCat::Pessoal,
                    embedido: true,
                }
            } else {
                main { class: "main",
                    h2 { class: "titulo-embed", "Catálogo base" }
                    p { class: "sub embed-sub", "Produtos partilhados pelo administrador (só leitura)" }
                    if let Some(e) = erro_base() {
                        p { class: "err", "{e}" }
                    }
                    section {
                        h2 { "Secção" }
                        div { class: "filtros-secao",
                            button {
                                class: if filtro_secao().is_none() { "chip on" } else { "chip" },
                                onclick: move |_| filtro_secao.set(None),
                                "Todas ({base().len()})"
                            }
                            for s in Secao::ALL {
                                {
                                    let cod = s as u8;
                                    let n = base().iter().filter(|a| a.secao == cod).count();
                                    if n == 0 {
                                        rsx! {}
                                    } else {
                                        rsx! {
                                            button {
                                                key: "{cod}",
                                                class: if filtro_secao() == Some(cod) { "chip on" } else { "chip" },
                                                onclick: move |_| filtro_secao.set(Some(cod)),
                                                "{s.label()} ({n})"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    section {
                        h2 { "Produtos ({filtrados_base.len()})" }
                        if !base_carregado() {
                            p { class: "muted", "A carregar…" }
                        } else if filtrados_base.is_empty() {
                            p { class: "muted",
                                if base().is_empty() {
                                    "O catálogo base ainda está vazio."
                                } else {
                                    "Nenhum produto nesta secção."
                                }
                            }
                        }
                        ul { class: "lista",
                            for a in filtrados_base {
                                {
                                    let thumb = if a.imag.is_empty() {
                                        None
                                    } else {
                                        Some(format!("{}/imgs/{}", api_url(), a.imag))
                                    };
                                    rsx! {
                                        li { class: "item",
                                            key: "{a.n_reg}",
                                            if let Some(src) = thumb {
                                                img { class: "thumb", src: "{src}", alt: "", loading: "lazy" }
                                            } else {
                                                span { class: "thumb placeholder", aria_hidden: "true" }
                                            }
                                            div { class: "item-body",
                                                strong { "{a.nome}" }
                                                span { class: "meta",
                                                    "{a.unidade_label()} · {a.secao_label()}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Listas de compras: criar, abrir, juntar produtos, marcar comprado.
#[component]
fn ComprasPainel(api_url: Signal<String>, sessao: SessaoLista, modo_ida: bool) -> Element {
    let token = use_signal(|| sessao.token.clone());
    let mut listas = use_signal(Vec::<ListaCompra>::new);
    let mut itens = use_signal(Vec::<ItemListaDto>::new);
    let mut catalogo = use_signal(Vec::<Artigo>::new);
    let mut lista_aberta = use_signal(|| Option::<u64>::None);
    let mut nome_nova = use_signal(String::new);
    let mut erro = use_signal(|| Option::<String>::None);
    let mut filtro_add = use_signal(String::new);
    let mut filtro_secao_add = use_signal(|| Option::<u8>::None);
    let mut mostrar_add = use_signal(|| false);
    let mut ocultar_comprados = use_signal(|| false);
    let mut lojas = use_signal(Vec::<Loja>::new);
    let mut lojas_estado = use_signal(|| 0u8); // 0=a carregar, 1=ok, 2=erro
    let mut loja_ida = use_signal(|| ler_loja_ida());
    let mut nome_nova_loja = use_signal(String::new);
    let mut a_criar_loja = use_signal(|| false);
    // Item escolhido para ajustar preço unitário (Compras).
    let mut item_foco = use_signal(|| Option::<u64>::None);
    // Override do preço unitário (cêntimos) enquanto se ajusta com ±.
    let mut preco_unit = use_signal(HashMap::<u64, u32>::new);
    let mut historico = use_signal(Vec::<CompraHistorico>::new);
    let mut historico_aberto = use_signal(|| false);
    let mut historico_a_carregar = use_signal(|| false);
    let mut artigo_foco = use_signal(|| Option::<u64>::None);
    let mut totais = use_signal(|| TotaisLista {
        n_produtos: 0,
        n_comprados: 0,
        total_cent: 0,
        n_com_preco: 0,
        estimado_cent: 0,
        n_estimado: 0,
        estimado_lista_cent: 0,
        n_estimado_lista: 0,
    });

    let mut carregar_lojas = move || {
        let url = api_url();
        let t = token();
        lojas_estado.set(0);
        spawn(async move {
            match listar_lojas(&url, &t).await {
                Ok(l) => {
                    lojas.set(l);
                    lojas_estado.set(1);
                }
                Err(e) => {
                    lojas.set(Vec::new());
                    lojas_estado.set(2);
                    erro.set(Some(format!("Lojas: {}", e.0)));
                }
            }
        });
    };

    let carregar_listas = move || {
        let url = api_url();
        let t = token();
        spawn(async move {
            match listar_listas(&url, &t).await {
                Ok(l) => listas.set(l),
                Err(e) => erro.set(Some(e.0)),
            }
        });
    };

    let carregar_itens = move |id: u64| {
        let url = api_url();
        let t = token();
        spawn(async move {
            match listar_itens(&url, &t, id).await {
                Ok(l) => itens.set(l),
                Err(e) => erro.set(Some(e.0)),
            }
            if let Ok(tot) = totais_lista(&url, &t, id).await {
                totais.set(tot);
            }
        });
    };

    // Garante ida activa; se não houver, cria (Registado parte do zero).
    let garantir_ida = move |lista_id: u64| {
        if !modo_ida {
            return;
        }
        let url = api_url();
        let t = token();
        let loja = loja_ida().unwrap_or(0);
        spawn(async move {
            match obter_ida(&url, &t, lista_id).await {
                Ok(Some(_)) => {}
                Ok(None) => {
                    if let Err(e) = nova_ida(&url, &t, lista_id, loja).await {
                        erro.set(Some(e.0));
                        return;
                    }
                }
                Err(e) => {
                    erro.set(Some(e.0));
                    return;
                }
            }
            if let Ok(tot) = totais_lista(&url, &t, lista_id).await {
                totais.set(tot);
            }
        });
    };

    let mut abrir_lista = move |id: u64| {
        lista_aberta.set(Some(id));
        item_foco.set(None);
        artigo_foco.set(None);
        preco_unit.set(HashMap::new());
        historico.set(Vec::new());
        historico_aberto.set(false);
        historico_a_carregar.set(false);
        carregar_itens(id);
        if modo_ida {
            carregar_lojas();
            garantir_ida(id);
        }
    };

    let mut carregar_historico = move |artigo_id: u64| {
        let url = api_url();
        let t = token();
        historico_a_carregar.set(true);
        historico.set(Vec::new());
        spawn(async move {
            match historico_precos_artigo(&url, &t, artigo_id).await {
                Ok(h) => historico.set(h),
                Err(_) => historico.set(Vec::new()),
            }
            historico_a_carregar.set(false);
        });
    };

    let mut focar_item = move |item_id: u64, artigo_id: u64, ref_unit: u32| {
        preco_unit.write().entry(item_id).or_insert(ref_unit);
        item_foco.set(Some(item_id));
        artigo_foco.set(Some(artigo_id));
        historico.set(Vec::new());
        historico_aberto.set(false);
        historico_a_carregar.set(false);
    };

    let carregar_catalogo_pick = move || {
        let url = api_url();
        let t = token();
        spawn(async move {
            let mut all = Vec::new();
            if let Ok(b) = listar_catalogo(&url, &t).await {
                all.extend(b);
            }
            if let Ok(m) = listar_meu_catalogo(&url, &t).await {
                all.extend(m);
            }
            all.sort_by(|a, b| {
                a.secao.cmp(&b.secao).then_with(|| a.nome.to_lowercase().cmp(&b.nome.to_lowercase()))
            });
            catalogo.set(all);
        });
    };

    use_effect(move || {
        carregar_listas();
        if modo_ida {
            carregar_lojas();
        }
    });

    let por_comprar = itens().iter().filter(|i| i.estado == 0 && i.qtd > 0).count();
    let comprados = itens().iter().filter(|i| i.estado == 1 && i.qtd > 0).count();

    // Itens agrupados por secção (ordem do hipermercado); dentro: por comprar → nome.
    let grupos_secao = {
        let ocultar = ocultar_comprados();
        let mut lista = itens();
        if ocultar {
            lista.retain(|i| i.estado == 0);
        }
        let mut out = Vec::<(u8, &'static str, Vec<ItemListaDto>)>::new();
        for s in Secao::ALL {
            let cod = s as u8;
            let mut g: Vec<ItemListaDto> = lista.iter().filter(|i| i.secao == cod).cloned().collect();
            if g.is_empty() {
                continue;
            }
            g.sort_by(|a, b| {
                a.estado
                    .cmp(&b.estado)
                    .then_with(|| a.nome.to_lowercase().cmp(&b.nome.to_lowercase()))
            });
            out.push((cod, s.label(), g));
        }
        out
    };

    let candidatos = {
        let q = filtro_add().trim().to_lowercase();
        let f = filtro_secao_add();
        catalogo()
            .into_iter()
            .filter(|a| f.map(|s| a.secao == s).unwrap_or(true))
            .filter(|a| q.is_empty() || a.nome.to_lowercase().contains(&q))
            .collect::<Vec<_>>()
    };

    rsx! {
        main { class: "main",
            h2 { class: "titulo-embed", if modo_ida { "Compras" } else { "Listas" } }
            p { class: "sub embed-sub",
                if modo_ida {
                    "No supermercado: loja, preços e marcar comprado"
                } else {
                    "Monta a lista e vê o total estimado"
                }
            }
            if let Some(e) = erro() {
                p { class: "err", "{e}" }
            }

            if let Some(lid) = lista_aberta() {
                {
                    let nome_l = listas()
                        .iter()
                        .find(|l| l.n_reg == lid)
                        .map(|l| l.nome.clone())
                        .unwrap_or_else(|| format!("Lista #{lid}"));
                    rsx! {
                        section { class: "card-form",
                            div { class: "row",
                                button {
                                    class: "btn-ghost",
                                    onclick: move |_| {
                                        lista_aberta.set(None);
                                        itens.set(Vec::new());
                                        totais.set(TotaisLista {
                                            n_produtos: 0,
                                            n_comprados: 0,
                                            total_cent: 0,
                                            n_com_preco: 0,
                                            estimado_cent: 0,
                                            n_estimado: 0,
                                            estimado_lista_cent: 0,
                                            n_estimado_lista: 0,
                                        });
                                        mostrar_add.set(false);
                                        filtro_secao_add.set(None);
                                        filtro_add.set(String::new());
                                    },
                                    "← Listas"
                                }
                                strong { class: "grow-label", "{nome_l}" }
                                span { class: "meta", "{por_comprar} por comprar · {comprados} ok" }
                            }
                            div { class: "totais-lista totais-compacto",
                                span {
                                    span { class: "totais-label", "Lista" }
                                    " "
                                    strong { "{totais().n_produtos}" }
                                    " produtos"
                                }
                                span { class: "totais-sep", "·" }
                                span {
                                    "Total estimado — "
                                    strong { "{cent_para_euros(totais().estimado_lista_cent)}" }
                                }
                            }
                        }
                        if modo_ida {
                            section { class: "card-form ida-loja",
                                div { class: "row lista-cab",
                                    h2 { "Nesta ida" }
                                    button {
                                        class: "btn-ghost",
                                        onclick: move |_| {
                                            let Some(lid) = lista_aberta() else { return };
                                            let url = api_url();
                                            let t = token();
                                            let loja = loja_ida().unwrap_or(0);
                                            spawn(async move {
                                                match nova_ida(&url, &t, lid, loja).await {
                                                    Ok(_) => {
                                                        if let Ok(tot) = totais_lista(&url, &t, lid).await {
                                                            totais.set(tot);
                                                        }
                                                        erro.set(None);
                                                    }
                                                    Err(e) => erro.set(Some(e.0)),
                                                }
                                            });
                                        },
                                        "Nova ida"
                                    }
                                }
                                p { class: "hint", "Toca num produto para ajustar o preço unitário (±1 cêntimo). No ✓ grava unitário × qtd. «Nova ida» zera o Registado." }
                                div { class: "totais-ida",
                                    span { class: "totais-label", "Registado" }
                                    strong { "{cent_para_euros(totais().total_cent)}" }
                                    span { class: "meta",
                                        if totais().n_com_preco > 0 {
                                            " · {totais().n_com_preco} com preço"
                                        } else {
                                            " · ainda sem preços nesta ida"
                                        }
                                    }
                                }
                                div { class: "filtros-secao",
                                    button {
                                        class: if loja_ida().is_none() { "chip on" } else { "chip" },
                                        onclick: move |_| {
                                            loja_ida.set(None);
                                            guardar_loja_ida(None);
                                        },
                                        "Sem loja"
                                    }
                                    for lj in lojas() {
                                        {
                                            let id = lj.n_reg;
                                            let nome = lj.nome.clone();
                                            let sel = loja_ida() == Some(id);
                                            rsx! {
                                                button {
                                                    key: "loja-{id}",
                                                    class: if sel { "chip on" } else { "chip" },
                                                    onclick: move |_| {
                                                        loja_ida.set(Some(id));
                                                        guardar_loja_ida(Some(id));
                                                    },
                                                    "{nome}"
                                                }
                                            }
                                        }
                                    }
                                    button {
                                        class: "chip",
                                        onclick: move |_| carregar_lojas(),
                                        "↻"
                                    }
                                }
                                if lojas_estado() == 0 {
                                    p { class: "muted", "A carregar lojas…" }
                                } else if lojas_estado() == 2 {
                                    p { class: "err", "Falha a carregar lojas — toca em ↻" }
                                } else if lojas().is_empty() {
                                    p { class: "muted", "Ainda sem lojas na base." }
                                } else {
                                    p { class: "meta", "{lojas().len()} lojas" }
                                }
                                div { class: "row nova-loja",
                                    input {
                                        class: "grow",
                                        r#type: "text",
                                        placeholder: "Nova loja…",
                                        maxlength: "24",
                                        value: "{nome_nova_loja()}",
                                        oninput: move |e| nome_nova_loja.set(e.value()),
                                        onkeydown: move |e| {
                                            if e.key() != Key::Enter || a_criar_loja() {
                                                return;
                                            }
                                            let nome = nome_nova_loja().trim().to_string();
                                            if nome.is_empty() {
                                                return;
                                            }
                                            let url = api_url();
                                            let t = token();
                                            a_criar_loja.set(true);
                                            spawn(async move {
                                                match criar_loja(&url, &t, &nome).await {
                                                    Ok(lj) => {
                                                        nome_nova_loja.set(String::new());
                                                        loja_ida.set(Some(lj.n_reg));
                                                        guardar_loja_ida(Some(lj.n_reg));
                                                        match listar_lojas(&url, &t).await {
                                                            Ok(l) => {
                                                                lojas.set(l);
                                                                lojas_estado.set(1);
                                                            }
                                                            Err(_) => {}
                                                        }
                                                        erro.set(None);
                                                    }
                                                    Err(e) => erro.set(Some(e.0)),
                                                }
                                                a_criar_loja.set(false);
                                            });
                                        },
                                    }
                                    button {
                                        class: "btn",
                                        disabled: a_criar_loja() || nome_nova_loja().trim().is_empty(),
                                        onclick: move |_| {
                                            if a_criar_loja() {
                                                return;
                                            }
                                            let nome = nome_nova_loja().trim().to_string();
                                            if nome.is_empty() {
                                                return;
                                            }
                                            let url = api_url();
                                            let t = token();
                                            a_criar_loja.set(true);
                                            spawn(async move {
                                                match criar_loja(&url, &t, &nome).await {
                                                    Ok(lj) => {
                                                        nome_nova_loja.set(String::new());
                                                        loja_ida.set(Some(lj.n_reg));
                                                        guardar_loja_ida(Some(lj.n_reg));
                                                        match listar_lojas(&url, &t).await {
                                                            Ok(l) => {
                                                                lojas.set(l);
                                                                lojas_estado.set(1);
                                                            }
                                                            Err(_) => {}
                                                        }
                                                        erro.set(None);
                                                    }
                                                    Err(e) => erro.set(Some(e.0)),
                                                }
                                                a_criar_loja.set(false);
                                            });
                                        },
                                        if a_criar_loja() { "…" } else { "Criar" }
                                    }
                                }
                            }
                        }
                        section {
                            div { class: "row lista-cab",
                                h2 { "Na lista ({itens().len()})" }
                                if comprados > 0 {
                                    button {
                                        class: if ocultar_comprados() { "chip on" } else { "chip" },
                                        onclick: move |_| ocultar_comprados.set(!ocultar_comprados()),
                                        if ocultar_comprados() { "Mostrar comprados" } else { "Só por comprar" }
                                    }
                                }
                            }
                            if modo_ida {
                                if let Some(fid) = item_foco() {
                                    {
                                        let focado = itens().into_iter().find(|i| i.n_reg == fid);
                                        rsx! {
                                            if let Some(it) = focado {
                                                {
                                                    let id = it.n_reg;
                                                    let qtd = it.qtd;
                                                    let uni = Unidade::from_u8(it.unidade).label();
                                                    let unit = preco_unit()
                                                        .get(&id)
                                                        .copied()
                                                        .unwrap_or(it.preco_ref_cent);
                                                    let linha = unit.saturating_mul(u32::from(qtd.max(1)));
                                                    let nome = it.nome.clone();
                                                    rsx! {
                                                        div { class: "preco-foco",
                                                            div { class: "preco-foco-cab",
                                                                strong { "{nome}" }
                                                                span { class: "meta", "{qtd} × {uni}" }
                                                                button {
                                                                    class: "btn-ghost",
                                                                    onclick: move |_| {
                                                                        item_foco.set(None);
                                                                        artigo_foco.set(None);
                                                                        historico_aberto.set(false);
                                                                        historico.set(Vec::new());
                                                                    },
                                                                    "Fechar"
                                                                }
                                                            }
                                                            div { class: "preco-foco-ajuste",
                                                                span { class: "totais-label", "Preço unitário" }
                                                                button {
                                                                    class: "btn-ghost qtd-btn preco-step",
                                                                    onclick: move |_| {
                                                                        let mut m = preco_unit();
                                                                        let actual = m.get(&id).copied().unwrap_or(unit);
                                                                        m.insert(id, actual.saturating_sub(1));
                                                                        preco_unit.set(m);
                                                                    },
                                                                    "−"
                                                                }
                                                                strong { class: "preco-foco-val", "{cent_para_curto(unit)} €" }
                                                                button {
                                                                    class: "btn-ghost qtd-btn preco-step",
                                                                    onclick: move |_| {
                                                                        let mut m = preco_unit();
                                                                        let actual = m.get(&id).copied().unwrap_or(unit);
                                                                        m.insert(id, actual.saturating_add(1).min(999_999));
                                                                        preco_unit.set(m);
                                                                    },
                                                                    "+"
                                                                }
                                                                button {
                                                                    class: "btn-ghost",
                                                                    title: "−10 cêntimos",
                                                                    onclick: move |_| {
                                                                        let mut m = preco_unit();
                                                                        let actual = m.get(&id).copied().unwrap_or(unit);
                                                                        m.insert(id, actual.saturating_sub(10));
                                                                        preco_unit.set(m);
                                                                    },
                                                                    "−10¢"
                                                                }
                                                                button {
                                                                    class: "btn-ghost",
                                                                    title: "+10 cêntimos",
                                                                    onclick: move |_| {
                                                                        let mut m = preco_unit();
                                                                        let actual = m.get(&id).copied().unwrap_or(unit);
                                                                        m.insert(id, actual.saturating_add(10).min(999_999));
                                                                        preco_unit.set(m);
                                                                    },
                                                                    "+10¢"
                                                                }
                                                            }
                                                            p { class: "meta preco-foco-linha",
                                                                "Linha ({qtd}×) — {cent_para_euros(linha)}"
                                                            }
                                                            div { class: "historico-precos",
                                                                div { class: "row lista-cab",
                                                                    button {
                                                                        class: if historico_aberto() { "chip on" } else { "chip" },
                                                                        onclick: move |_| {
                                                                            if historico_aberto() {
                                                                                historico_aberto.set(false);
                                                                                return;
                                                                            }
                                                                            historico_aberto.set(true);
                                                                            if let Some(aid) = artigo_foco() {
                                                                                carregar_historico(aid);
                                                                            }
                                                                        },
                                                                        if historico_aberto() { "Ocultar histórico" } else { "Histórico" }
                                                                    }
                                                                }
                                                                if historico_aberto() {
                                                                    if historico_a_carregar() {
                                                                        p { class: "muted", "A carregar histórico…" }
                                                                    } else if historico().is_empty() {
                                                                        p { class: "muted", "Ainda sem compras com preço." }
                                                                    } else {
                                                                        ul { class: "historico-lista",
                                                                            for h in historico() {
                                                                                {
                                                                                    let unit_h = h.preco_unit_cent;
                                                                                    let loja_id_h = h.loja_id;
                                                                                    let loja_h = h.loja.clone();
                                                                                    let data_h = data_curta(h.comprado_em);
                                                                                    let qtd_h = h.qtd;
                                                                                    let key = h.n_reg;
                                                                                    rsx! {
                                                                                        li {
                                                                                            key: "{key}",
                                                                                            button {
                                                                                                class: "historico-item",
                                                                                                title: "Usar este preço unitário",
                                                                                                onclick: move |_| {
                                                                                                    let mut m = preco_unit();
                                                                                                    m.insert(id, unit_h);
                                                                                                    preco_unit.set(m);
                                                                                                    if loja_id_h > 0 {
                                                                                                        loja_ida.set(Some(loja_id_h));
                                                                                                        guardar_loja_ida(Some(loja_id_h));
                                                                                                    }
                                                                                                },
                                                                                                span { class: "historico-loja", "{loja_h}" }
                                                                                                span { class: "historico-preco", "{cent_para_curto(unit_h)} €/un" }
                                                                                                span { class: "meta",
                                                                                                    if qtd_h > 1 {
                                                                                                        "×{qtd_h} · {data_h}"
                                                                                                    } else {
                                                                                                        "{data_h}"
                                                                                                    }
                                                                                                }
                                                                                            }
                                                                                        }
                                                                                    }
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if itens().is_empty() {
                                p { class: "muted", "Lista vazia — adiciona produtos abaixo." }
                            } else if grupos_secao.is_empty() {
                                p { class: "muted", "Tudo comprado — mostra de novo ou adiciona produtos." }
                            }
                            for (cod, label, grupo) in grupos_secao {
                                {
                                    let n_grupo = grupo.len();
                                    let n_ok = grupo.iter().filter(|i| i.estado == 1).count();
                                    rsx! {
                                        div { class: "secao-grupo",
                                            key: "g-{cod}",
                                            h3 {
                                                "{label}"
                                                span { class: "meta",
                                                    if n_ok > 0 {
                                                        " · {n_ok}/{n_grupo}"
                                                    } else {
                                                        " · {n_grupo}"
                                                    }
                                                }
                                            }
                                            ul { class: "lista",
                                                for it in grupo {
                                                    {
                                                        let id = it.n_reg;
                                                        let feito = it.estado == 1;
                                                        let thumb = if it.imag.is_empty() {
                                                            None
                                                        } else {
                                                            Some(format!("{}/imgs/{}", api_url(), it.imag))
                                                        };
                                                        let uni = Unidade::from_u8(it.unidade).label();
                                                        let qtd = it.qtd;
                                                        let zero = qtd == 0;
                                                        let ref_unit = it.preco_ref_cent;
                                                        let artigo_id = it.artigo_id;
                                                        let unit = preco_unit()
                                                            .get(&id)
                                                            .copied()
                                                            .unwrap_or(ref_unit);
                                                        let focado = item_foco() == Some(id);
                                                        rsx! {
                                                            li {
                                                                class: if feito {
                                                                    "item risc"
                                                                } else if zero {
                                                                    "item zero"
                                                                } else if focado {
                                                                    "item foco"
                                                                } else {
                                                                    "item"
                                                                },
                                                                key: "{id}",
                                                                if let Some(src) = thumb {
                                                                    img {
                                                                        class: "thumb",
                                                                        src: "{src}",
                                                                        alt: "",
                                                                        loading: "lazy",
                                                                        onclick: move |_| {
                                                                            if modo_ida && !feito && !zero {
                                                                                focar_item(id, artigo_id, ref_unit);
                                                                            }
                                                                        },
                                                                    }
                                                                } else {
                                                                    span {
                                                                        class: "thumb placeholder",
                                                                        aria_hidden: "true",
                                                                        onclick: move |_| {
                                                                            if modo_ida && !feito && !zero {
                                                                                focar_item(id, artigo_id, ref_unit);
                                                                            }
                                                                        },
                                                                    }
                                                                }
                                                                div {
                                                                    class: "item-body",
                                                                    onclick: move |_| {
                                                                        if modo_ida && !feito && !zero {
                                                                            focar_item(id, artigo_id, ref_unit);
                                                                        }
                                                                    },
                                                                    strong { "{it.nome}" }
                                                                    span { class: "meta",
                                                                        if zero {
                                                                            "off · {uni}"
                                                                        } else if modo_ida {
                                                                            if unit > 0 {
                                                                                "{uni} · {cent_para_curto(unit)} €/un"
                                                                            } else {
                                                                                "{uni} · sem ref."
                                                                            }
                                                                        } else {
                                                                            "{uni}"
                                                                        }
                                                                    }
                                                                }
                                                                div { class: "item-actions",
                                                                    div { class: "qtd-step",
                                                                        button {
                                                                            class: "btn-ghost qtd-btn",
                                                                            disabled: feito || qtd == 0,
                                                                            onclick: move |_| {
                                                                                if feito || qtd == 0 { return; }
                                                                                let url = api_url();
                                                                                let t = token();
                                                                                let nova = qtd - 1;
                                                                                spawn(async move {
                                                                                    let p = PedidoItemPatch {
                                                                                        qtd: Some(nova),
                                                                                        ..Default::default()
                                                                                    };
                                                                                    match editar_item(&url, &t, id, &p).await {
                                                                                        Ok(_) => carregar_itens(lid),
                                                                                        Err(e) => erro.set(Some(e.0)),
                                                                                    }
                                                                                });
                                                                            },
                                                                            "−"
                                                                        }
                                                                        span { class: "qtd-val", "{qtd}" }
                                                                        button {
                                                                            class: "btn-ghost qtd-btn",
                                                                            disabled: feito,
                                                                            onclick: move |_| {
                                                                                if feito { return; }
                                                                                let url = api_url();
                                                                                let t = token();
                                                                                let nova = qtd.saturating_add(1);
                                                                                spawn(async move {
                                                                                    let p = PedidoItemPatch {
                                                                                        qtd: Some(nova),
                                                                                        ..Default::default()
                                                                                    };
                                                                                    match editar_item(&url, &t, id, &p).await {
                                                                                        Ok(_) => carregar_itens(lid),
                                                                                        Err(e) => erro.set(Some(e.0)),
                                                                                    }
                                                                                });
                                                                            },
                                                                            "+"
                                                                        }
                                                                    }
                                                                    button {
                                                                        class: "btn-ghost",
                                                                        disabled: zero,
                                                                        onclick: move |_| {
                                                                            if zero { return; }
                                                                            let url = api_url();
                                                                            let t = token();
                                                                            let novo = if feito { 0 } else { 1 };
                                                                            let loja = if novo == 1 { loja_ida() } else { None };
                                                                            let preco = if novo == 1 {
                                                                                let u = preco_unit()
                                                                                    .get(&id)
                                                                                    .copied()
                                                                                    .unwrap_or(ref_unit);
                                                                                Some(u.saturating_mul(u32::from(qtd)))
                                                                            } else {
                                                                                None
                                                                            };
                                                                            spawn(async move {
                                                                                let p = PedidoItemPatch {
                                                                                    qtd: None,
                                                                                    estado: Some(novo),
                                                                                    loja_id: loja,
                                                                                    preco_cent: preco,
                                                                                };
                                                                                match editar_item(&url, &t, id, &p).await {
                                                                                    Ok(_) => {
                                                                                        if novo == 1 {
                                                                                            preco_unit.write().remove(&id);
                                                                                            if item_foco() == Some(id) {
                                                                                                item_foco.set(None);
                                                                                            }
                                                                                        }
                                                                                        carregar_itens(lid);
                                                                                    }
                                                                                    Err(e) => erro.set(Some(e.0)),
                                                                                }
                                                                            });
                                                                        },
                                                                        if feito { "↩" } else { "✓" }
                                                                    }
                                                                    button {
                                                                        class: "btn-ghost danger",
                                                                        onclick: move |_| {
                                                                            let url = api_url();
                                                                            let t = token();
                                                                            spawn(async move {
                                                                                match remover_item(&url, &t, id).await {
                                                                                    Ok(()) => {
                                                                                        if item_foco() == Some(id) {
                                                                                            item_foco.set(None);
                                                                                        }
                                                                                        carregar_itens(lid);
                                                                                    }
                                                                                    Err(e) => erro.set(Some(e.0)),
                                                                                }
                                                                            });
                                                                        },
                                                                        "×"
                                                                    }
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        section { class: "card-form",
                            h2 { "Adicionar produto" }
                            if !mostrar_add() {
                                button {
                                    class: "btn",
                                    onclick: move |_| {
                                        mostrar_add.set(true);
                                        carregar_catalogo_pick();
                                    },
                                    "Escolher do catálogo…"
                                }
                            } else {
                                h2 { "Secção" }
                                div { class: "filtros-secao",
                                    button {
                                        class: if filtro_secao_add().is_none() { "chip on" } else { "chip" },
                                        onclick: move |_| filtro_secao_add.set(None),
                                        "Todas ({catalogo().len()})"
                                    }
                                    for s in Secao::ALL {
                                        {
                                            let cod = s as u8;
                                            let n = catalogo().iter().filter(|a| a.secao == cod).count();
                                            if n == 0 {
                                                rsx! {}
                                            } else {
                                                rsx! {
                                                    button {
                                                        key: "s-{cod}",
                                                        class: if filtro_secao_add() == Some(cod) { "chip on" } else { "chip" },
                                                        onclick: move |_| filtro_secao_add.set(Some(cod)),
                                                        "{s.label()} ({n})"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                input {
                                    class: "grow",
                                    style: "margin-top: 0.75rem; width: 100%;",
                                    placeholder: "Filtrar por nome…",
                                    value: "{filtro_add}",
                                    oninput: move |e| filtro_add.set(e.value()),
                                }
                                h2 { style: "margin-top: 0.85rem;", "Produtos ({candidatos.len()})" }
                                if candidatos.is_empty() {
                                    p { class: "muted",
                                        if catalogo().is_empty() {
                                            "A carregar catálogo…"
                                        } else {
                                            "Nenhum produto nesta secção."
                                        }
                                    }
                                }
                                ul { class: "lista pick",
                                    for a in candidatos {
                                        {
                                            let aid = a.n_reg;
                                            let thumb = if a.imag.is_empty() {
                                                None
                                            } else {
                                                Some(format!("{}/imgs/{}", api_url(), a.imag))
                                            };
                                            rsx! {
                                                li { class: "item",
                                                    key: "p-{aid}",
                                                    if let Some(src) = thumb {
                                                        img { class: "thumb", src: "{src}", alt: "", loading: "lazy" }
                                                    } else {
                                                        span { class: "thumb placeholder", aria_hidden: "true" }
                                                    }
                                                    div { class: "item-body",
                                                        strong { "{a.nome}" }
                                                        span { class: "meta", "{a.unidade_label()} · {a.secao_label()}" }
                                                    }
                                                    button {
                                                        class: "btn",
                                                        onclick: move |_| {
                                                            let url = api_url();
                                                            let t = token();
                                                            spawn(async move {
                                                                match adicionar_item(&url, &t, lid, aid, 1).await {
                                                                    Ok(_) => carregar_itens(lid),
                                                                    Err(e) => erro.set(Some(e.0)),
                                                                }
                                                            });
                                                        },
                                                        "+"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                button {
                                    class: "btn-ghost",
                                    onclick: move |_| {
                                        mostrar_add.set(false);
                                        filtro_secao_add.set(None);
                                        filtro_add.set(String::new());
                                    },
                                    "Fechar catálogo"
                                }
                            }
                        }
                    }
                }
            } else {
                section { class: "card-form",
                    h2 { "Nova lista" }
                    div { class: "row",
                        input {
                            class: "grow",
                            placeholder: "ex.: Compras do sábado",
                            maxlength: "24",
                            value: "{nome_nova}",
                            oninput: move |e| nome_nova.set(e.value()),
                        }
                        button {
                            class: "btn",
                            onclick: move |_| {
                                let n = nome_nova().trim().to_string();
                                if n.is_empty() {
                                    erro.set(Some("Indica o nome da lista.".into()));
                                    return;
                                }
                                let url = api_url();
                                let t = token();
                                spawn(async move {
                                    match criar_lista(&url, &t, &n).await {
                                        Ok(l) => {
                                            nome_nova.set(String::new());
                                            carregar_listas();
                                            abrir_lista(l.n_reg);
                                        }
                                        Err(e) => erro.set(Some(e.0)),
                                    }
                                });
                            },
                            "Criar"
                        }
                    }
                }
                section {
                    h2 { "As tuas listas ({listas().len()})" }
                    if listas().is_empty() {
                        p { class: "muted", "Ainda sem listas — cria a primeira." }
                    }
                    ul { class: "lista",
                        for l in listas() {
                            {
                                let id = l.n_reg;
                                rsx! {
                                    li { class: "item",
                                        key: "{id}",
                                        div { class: "item-body",
                                            strong { "{l.nome}" }
                                        }
                                        div { class: "item-actions",
                                            button {
                                                class: "btn",
                                                onclick: move |_| {
                                                    abrir_lista(id);
                                                },
                                                "Abrir"
                                            }
                                            button {
                                                class: "btn-ghost danger",
                                                onclick: move |_| {
                                                    let url = api_url();
                                                    let t = token();
                                                    spawn(async move {
                                                        match remover_lista(&url, &t, id).await {
                                                            Ok(()) => carregar_listas(),
                                                            Err(e) => erro.set(Some(e.0)),
                                                        }
                                                    });
                                                },
                                                "Apagar"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum ModoCat {
    /// Admin — CRUD do catálogo geral (utilizador = 1).
    BaseAdmin,
    /// Utilizador — CRUD dos seus produtos.
    Pessoal,
    /// Utilizador — ver o catálogo geral, sem editar.
    BaseLeitura,
}

impl ModoCat {
    fn titulo(self) -> &'static str {
        match self {
            Self::BaseAdmin => "Lista · Catálogo base",
            Self::Pessoal => "Os meus produtos",
            Self::BaseLeitura => "Catálogo base",
        }
    }

    fn pode_editar(self) -> bool {
        !matches!(self, Self::BaseLeitura)
    }

    fn e_pessoal(self) -> bool {
        matches!(self, Self::Pessoal)
    }
}

#[component]
fn CatalogoPainel(
    api_url: Signal<String>,
    sessao: SessaoLista,
    on_sair: EventHandler<()>,
    modo: ModoCat,
    embedido: bool,
) -> Element {
    let token = use_signal(|| sessao.token.clone());
    let mut itens = use_signal(Vec::<Artigo>::new);
    let mut erro = use_signal(|| Option::<String>::None);
    let mut nome = use_signal(String::new);
    let mut imag = use_signal(String::new);
    let mut unidade = use_signal(|| Unidade::Un as u8);
    let mut secao = use_signal(|| Secao::Mercearia as u8);
    let mut filtro_secao = use_signal(|| Option::<u8>::None);
    let mut a_gravar = use_signal(|| false);
    let mut editar_id = use_signal(|| Option::<u64>::None);
    let mut imag_rev = use_signal(|| 0u64);
    let mut a_carregar_img = use_signal(|| false);

    let carregar = move || {
        let url = api_url();
        let t = token();
        let m = modo;
        spawn(async move {
            let res = if m.e_pessoal() {
                listar_meu_catalogo(&url, &t).await
            } else {
                listar_catalogo(&url, &t).await
            };
            match res {
                Ok(l) => { itens.set(l); erro.set(None); }
                Err(e) => {
                    itens.set(Vec::new());
                    erro.set(Some(e.0));
                }
            }
        });
    };

    // Recarrega quando o modo muda (ex.: aba Os meus ↔ Catálogo base).
    use_effect(move || {
        let _modo = modo;
        carregar();
    });

    let gravar = move |_| {
        if !modo.pode_editar() {
            return;
        }
        let n = nome().trim().to_string();
        if n.is_empty() {
            erro.set(Some("Indica o nome do produto.".into()));
            return;
        }
        let pedido = PedidoArtigo {
            nome: n,
            imag: imag().trim().to_string(),
            unidade: unidade(),
            secao: secao(),
        };
        let url = api_url();
        let t = token();
        let id_opt = editar_id();
        let pessoal = modo.e_pessoal();
        a_gravar.set(true);
        spawn(async move {
            let res = if let Some(id) = id_opt {
                if pessoal {
                    editar_meu_catalogo(&url, &t, id, &pedido).await.map(|_| ())
                } else {
                    editar_catalogo(&url, &t, id, &pedido).await.map(|_| ())
                }
            } else if pessoal {
                criar_meu_catalogo(&url, &t, &pedido).await.map(|_| ())
            } else {
                criar_catalogo(&url, &t, &pedido).await.map(|_| ())
            };
            a_gravar.set(false);
            match res {
                Ok(()) => {
                    nome.set(String::new());
                    imag.set(String::new());
                    editar_id.set(None);
                    carregar();
                }
                Err(e) => erro.set(Some(e.0)),
            }
        });
    };

    let filtrados = {
        let f = filtro_secao();
        itens()
            .into_iter()
            .filter(|a| f.map(|s| a.secao == s).unwrap_or(true))
            .collect::<Vec<_>>()
    };

    let sub = match modo {
        ModoCat::BaseAdmin => format!("Admin {} — produtos para todos os utilizadores", sessao.nome),
        ModoCat::Pessoal => format!("Só tu vês e editas estes produtos (id {})", sessao.labnetcol_id),
        ModoCat::BaseLeitura => "Produtos partilhados pelo administrador (só leitura)".into(),
    };

    let corpo = rsx! {
        main { class: "main",
            if let Some(e) = erro() {
                p { class: "err", "{e}" }
            }
            if modo.pode_editar() {
                section { class: "card-form",
                    h2 { if editar_id().is_some() { "Editar produto" } else { "Novo produto" } }
                    div { class: "form-imag",
                        {
                            let nome_img = imag().trim().to_string();
                            let rev = imag_rev();
                            if nome_img.is_empty() {
                                rsx! {
                                    div { class: "preview-imag vazio",
                                        span { "Sem imagem" }
                                    }
                                }
                            } else {
                                let src = format!("{}/imgs/{}?v={}", api_url(), nome_img, rev);
                                rsx! {
                                    img {
                                        class: "preview-imag",
                                        src: "{src}",
                                        alt: "Pré-visualização",
                                    }
                                }
                            }
                        }
                        div { class: "form-imag-campos",
                            label { class: "label-imag", "Ficheiro da imagem" }
                            input {
                                class: "imag",
                                placeholder: "ex. imag12.png",
                                maxlength: "16",
                                value: "{imag}",
                                oninput: move |e| imag.set(e.value()),
                            }
                            label { class: "btn-file",
                                if a_carregar_img() { "A enviar…" } else { "Escolher / substituir ficheiro…" }
                                input {
                                    r#type: "file",
                                    accept: "image/png,image/jpeg,image/webp,image/gif",
                                    disabled: a_carregar_img(),
                                    onchange: move |ev| {
                                        let mut ficheiros = ev.files();
                                        let Some(file) = ficheiros.pop() else { return };
                                        let url = api_url();
                                        let t = token();
                                        let alvo = imag().trim().to_string();
                                        let id_edit = editar_id();
                                        let admin_modo = matches!(modo, ModoCat::BaseAdmin);
                                        a_carregar_img.set(true);
                                        spawn(async move {
                                            let nome_f = file.name();
                                            let bytes = match file.read_bytes().await {
                                                Ok(b) => b,
                                                Err(_) => {
                                                    a_carregar_img.set(false);
                                                    erro.set(Some("Não foi possível ler o ficheiro.".into()));
                                                    return;
                                                }
                                            };
                                            let ext = nome_f
                                                .rsplit('.')
                                                .next()
                                                .unwrap_or("png")
                                                .to_ascii_lowercase();
                                            let b64 = encode_b64(&bytes);
                                            // Pessoal: sempre nome novo (não sobrescreve ícones base).
                                            // Admin: pode substituir pelo nome actual / imag{id}.png
                                            let nome_opt = if admin_modo {
                                                if !alvo.is_empty() {
                                                    Some(alvo)
                                                } else {
                                                    id_edit.map(|id| format!("imag{id}.png"))
                                                }
                                            } else {
                                                None
                                            };
                                            match upload_imagem(
                                                &url,
                                                &t,
                                                &b64,
                                                &ext,
                                                nome_opt.as_deref(),
                                            ).await {
                                                Ok(r) => {
                                                    imag.set(r.imag);
                                                    imag_rev.set(imag_rev() + 1);
                                                    erro.set(None);
                                                }
                                                Err(e) => erro.set(Some(e.0)),
                                            }
                                            a_carregar_img.set(false);
                                        });
                                    },
                                }
                            }
                            p { class: "hint",
                                if editar_id().is_some() {
                                    "A pré-visualização actualiza ao mudar o nome ou ao substituir o ficheiro."
                                } else {
                                    "Em branco no novo produto → grava-se imag{{n_reg}}.png automaticamente."
                                }
                            }
                        }
                    }
                    div { class: "row",
                        input {
                            class: "grow",
                            placeholder: "Nome (máx. 24)",
                            maxlength: "24",
                            value: "{nome}",
                            oninput: move |e| nome.set(e.value()),
                        }
                        select {
                            value: "{unidade}",
                            onchange: move |e| {
                                if let Ok(n) = e.value().parse() { unidade.set(n); }
                            },
                            option { value: "0", "un" }
                            option { value: "1", "kg" }
                            option { value: "2", "g" }
                            option { value: "3", "L" }
                            option { value: "4", "ml" }
                            option { value: "5", "pack" }
                        }
                        select {
                            value: "{secao}",
                            onchange: move |e| {
                                if let Ok(n) = e.value().parse() { secao.set(n); }
                            },
                            for s in Secao::ALL {
                                option { value: "{s as u8}", "{s.label()}" }
                            }
                        }
                        button {
                            class: "btn",
                            disabled: a_gravar(),
                            onclick: gravar,
                            if a_gravar() { "…" } else if editar_id().is_some() { "Guardar" } else { "Adicionar" }
                        }
                        if editar_id().is_some() {
                            button {
                                class: "btn-ghost",
                                onclick: move |_| {
                                    editar_id.set(None);
                                    nome.set(String::new());
                                    imag.set(String::new());
                                },
                                "Cancelar"
                            }
                        }
                    }
                }
            }
            section {
                h2 { "Secção" }
                div { class: "filtros-secao",
                    button {
                        class: if filtro_secao().is_none() { "chip on" } else { "chip" },
                        onclick: move |_| filtro_secao.set(None),
                        "Todas ({itens().len()})"
                    }
                    for s in Secao::ALL {
                        {
                            let cod = s as u8;
                            let n = itens().iter().filter(|a| a.secao == cod).count();
                            if n == 0 {
                                rsx! {}
                            } else {
                                rsx! {
                                    button {
                                        key: "{cod}",
                                        class: if filtro_secao() == Some(cod) { "chip on" } else { "chip" },
                                        onclick: move |_| filtro_secao.set(Some(cod)),
                                        "{s.label()} ({n})"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            section {
                h2 { "Produtos ({filtrados.len()})" }
                if filtrados.is_empty() {
                    p { class: "muted",
                        if itens().is_empty() {
                            if modo.pode_editar() {
                                "Ainda sem produtos — adiciona o primeiro."
                            } else {
                                "O catálogo base ainda está vazio."
                            }
                        } else {
                            "Nenhum produto nesta secção."
                        }
                    }
                }
                ul { class: "lista",
                    for a in filtrados {
                        {
                            let id = a.n_reg;
                            let nome_a = a.nome.clone();
                            let imag_a = a.imag.clone();
                            let uni = a.unidade;
                            let sec = a.secao;
                            let thumb = if a.imag.is_empty() {
                                None
                            } else {
                                Some(format!("{}/imgs/{}", api_url(), a.imag))
                            };
                            let pessoal = modo.e_pessoal();
                            rsx! {
                                li { class: "item",
                                    key: "{id}",
                                    if let Some(src) = thumb {
                                        img { class: "thumb", src: "{src}", alt: "", loading: "lazy" }
                                    } else {
                                        span { class: "thumb placeholder", aria_hidden: "true" }
                                    }
                                    div { class: "item-body",
                                        strong { "{a.nome}" }
                                        span { class: "meta",
                                            "{a.unidade_label()} · {a.secao_label()}"
                                            if !a.imag.is_empty() {
                                                span { " · {a.imag}" }
                                            }
                                        }
                                    }
                                    if modo.pode_editar() {
                                        div { class: "item-actions",
                                            button {
                                                class: "btn-ghost",
                                                onclick: move |_| {
                                                    editar_id.set(Some(id));
                                                    nome.set(nome_a.clone());
                                                    imag.set(imag_a.clone());
                                                    unidade.set(uni);
                                                    secao.set(sec);
                                                    imag_rev.set(imag_rev() + 1);
                                                },
                                                "Editar"
                                            }
                                            button {
                                                class: "btn-ghost danger",
                                                onclick: move |_| {
                                                    let url = api_url();
                                                    let t = token();
                                                    spawn(async move {
                                                        let res = if pessoal {
                                                            remover_meu_catalogo(&url, &t, id).await
                                                        } else {
                                                            remover_catalogo(&url, &t, id).await
                                                        };
                                                        match res {
                                                            Ok(()) => carregar(),
                                                            Err(e) => erro.set(Some(e.0)),
                                                        }
                                                    });
                                                },
                                                "Apagar"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    if embedido {
        rsx! {
            div { class: "painel-embed",
                if !matches!(modo, ModoCat::BaseAdmin) {
                    h2 { class: "titulo-embed", "{modo.titulo()}" }
                    p { class: "sub embed-sub", "{sub}" }
                }
                {corpo}
            }
        }
    } else {
        rsx! {
            div { class: "app",
                header { class: "top",
                    div {
                        h1 { "{modo.titulo()}" }
                        p { class: "sub", "{sub}" }
                    }
                    button { class: "btn-ghost", onclick: move |_| on_sair.call(()), "Sair" }
                }
                {corpo}
            }
        }
    }
}
