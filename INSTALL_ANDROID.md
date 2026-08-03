# Instalar a Lista no Android

APK (~9 MB): na página **Downloads** do LabNetCol, ou directamente:

`https://teste.labnetcol.pt/downloads/dio-lista.apk`

## Passos

1. Abre o link no **browser do telemóvel** (Chrome, Firefox, etc.).
2. Descarrega o ficheiro `dio-lista.apk`.
3. Ao abrir o APK, o Android pede permissões — é normal aceitar **2 ou 3 vezes**:
   - permitir instalar a partir deste browser / “fontes desconhecidas”;
   - confirmar a instalação;
   - por vezes um aviso extra de segurança (Xiaomi/MIUI, etc.).
4. Se a app **já estiver instalada**, o Android pergunta se queres **reinstalar** / actualizar — também é normal.
5. Quando aparecer **Aplicação instalada**, abre **Lista**.
6. Na etiqueta **BD:** deve dizer `mcs_bd2` (não localStorage).

O ficheiro fica nas **Transferências** do browser (pode não abrir um ecrã novo ao clicar em Descarregar).

## Testar se grava de verdade

1. Adiciona 1–3 produtos.
2. Fecha a app (ou reinicia o telemóvel).
3. Volta a abrir **Lista** — os produtos devem continuar.

## Notas

- Não passa pela Play Store: é instalação directa (sideload).
- Só **arm64** (telemóveis modernos). Emuladores x86 podem não servir.
- O primeiro arranque pode levar **3–5 segundos**; arranques seguintes costumam ser mais rápidos se a app ainda estiver em memória.
- Pacote: `pt.mcs.lista`
