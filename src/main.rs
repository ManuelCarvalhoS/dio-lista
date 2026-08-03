#![allow(non_snake_case)]

mod store;

#[cfg(not(target_arch = "wasm32"))]
mod bd;

use dioxus::prelude::*;
use lista_comum::{Artigo, Secao, Unidade, UTILIZADOR_BASE};
use store::{adicionar, backend_label, guardar_imagem, listar_ou_vazio, url_imagem};

const CSS: Asset = asset!("/assets/main.css");

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let mut nome = use_signal(String::new);
    let mut unidade = use_signal(|| Unidade::Un as u8);
    let mut secao = use_signal(|| Secao::Mercearia as u8);
    let mut imag = use_signal(String::new);
    let mut imag_preview = use_signal(|| Option::<String>::None);

    let mut artigos = use_signal(listar_ou_vazio);
    let mut erro = use_signal(|| Option::<String>::None);
    let mut ok = use_signal(|| Option::<String>::None);
    let backend = backend_label();

    let mut guardar = move |_| {
        erro.set(None);
        ok.set(None);

        let mut a = Artigo::novo_base(&nome());
        a.utilizador = UTILIZADOR_BASE;
        a.unidade = unidade();
        a.secao = secao();
        a.imag = imag().trim().to_string();

        match adicionar(a) {
            Ok(p) => {
                ok.set(Some(format!(
                    "Guardado: {} · {} · {} (#{})",
                    p.nome,
                    p.unidade_label(),
                    p.secao_label(),
                    p.n_reg
                )));
                nome.set(String::new());
                imag.set(String::new());
                imag_preview.set(None);
                artigos.set(listar_ou_vazio());
            }
            Err(e) => erro.set(Some(e)),
        }
    };

    rsx! {
        document::Stylesheet { href: CSS }
        document::Link {
            rel: "stylesheet",
            href: "https://fonts.googleapis.com/css2?family=DM+Sans:wght@400;500;600;700&family=Fraunces:opsz,wght@9..144,600;9..144,700&display=swap",
        }
        document::Title { "Lista — catálogo" }

        div { class: "shell",
            header { class: "top",
                h1 { class: "brand", "Lista" }
                p { class: "tagline", "Catálogo (mesmo modelo que o servidor)" }
            }

            section { class: "panel",
                p { class: "backend",
                    "BD: " span { class: "backend-val", "{backend}" }
                }

                form {
                    class: "form",
                    onsubmit: move |ev| {
                        ev.prevent_default();
                        guardar(());
                    },

                    label { class: "label", r#for: "nome", "Produto" }
                    input {
                        id: "nome",
                        class: "input",
                        r#type: "text",
                        maxlength: "24",
                        placeholder: "ex.: leite meio-gordo",
                        value: "{nome}",
                        oninput: move |ev| nome.set(ev.value()),
                        autofocus: true,
                    }

                    label { class: "label", "Imagem (opcional, ≤500 KB)" }
                    div { class: "img-row",
                        if let Some(src) = imag_preview() {
                            img { class: "thumb", src: "{src}", alt: "preview" }
                        } else {
                            div { class: "thumb thumb-empty", "—" }
                        }
                        input {
                            class: "input",
                            r#type: "file",
                            accept: "image/*",
                            onchange: move |ev| {
                                let files = ev.files();
                                let Some(file) = files.into_iter().next() else { return };
                                spawn(async move {
                                    match file.read_bytes().await {
                                        Ok(bytes) => match guardar_imagem(&bytes) {
                                            Ok(id) => {
                                                imag_preview.set(url_imagem(&id));
                                                imag.set(id);
                                                erro.set(None);
                                            }
                                            Err(e) => erro.set(Some(e)),
                                        },
                                        Err(_) => {
                                            erro.set(Some("Não foi possível ler a imagem.".into()));
                                        }
                                    }
                                });
                            },
                        }
                    }

                    div { class: "row",
                        div { class: "field",
                            label { class: "label", r#for: "un", "Unidade" }
                            select {
                                id: "un",
                                class: "input input-sm",
                                onchange: move |ev| {
                                    if let Ok(n) = ev.value().parse::<u8>() {
                                        unidade.set(n);
                                    }
                                },
                                option { value: "0", selected: unidade() == 0, "un" }
                                option { value: "1", selected: unidade() == 1, "kg" }
                                option { value: "2", selected: unidade() == 2, "g" }
                                option { value: "3", selected: unidade() == 3, "L" }
                                option { value: "4", selected: unidade() == 4, "ml" }
                                option { value: "5", selected: unidade() == 5, "pack" }
                            }
                        }
                        div { class: "field",
                            label { class: "label", r#for: "secao", "Secção" }
                            select {
                                id: "secao",
                                class: "input",
                                onchange: move |ev| {
                                    if let Ok(n) = ev.value().parse::<u8>() {
                                        secao.set(n);
                                    }
                                },
                                for s in Secao::ALL {
                                    option {
                                        value: "{s as u8}",
                                        selected: secao() == s as u8,
                                        "{s.label()}"
                                    }
                                }
                            }
                        }
                    }

                    button { class: "btn", r#type: "submit", "Adicionar" }
                }

                if let Some(e) = erro() {
                    p { class: "msg msg-erro", "{e}" }
                }
                if let Some(m) = ok() {
                    p { class: "msg msg-ok", "{m}" }
                }
            }

            section { class: "lista",
                h2 {
                    "Catálogo "
                    span { class: "count", "({artigos().len()})" }
                }
                if artigos().is_empty() {
                    p { class: "vazio", "Ainda sem produtos. Adiciona o primeiro." }
                } else {
                    ul { class: "items",
                        for a in artigos() {
                            {
                                let thumb = url_imagem(&a.imag);
                                rsx! {
                                    li { class: "item",
                                        if let Some(src) = thumb {
                                            img { class: "thumb-sm", src: "{src}", alt: "" }
                                        } else {
                                            div { class: "thumb-sm thumb-empty", "" }
                                        }
                                        div { class: "item-main",
                                            span { class: "item-nome", "{a.nome}" }
                                            span { class: "item-meta",
                                                "{a.unidade_label()} · {a.secao_label()}"
                                                if !a.imag.is_empty() {
                                                    " · {a.imag}"
                                                }
                                            }
                                        }
                                        span { class: "item-id", "#{a.n_reg}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            footer { class: "foot",
                "MCS Lab · lista_comum::Artigo (64 B) · igual ao servidor"
            }
        }
    }
}
