# Discard

Un client Windows léger pour **Discord Web**, avec un hôte **Rust + Win32** et
**Microsoft Edge WebView2**. Le moteur WebView2 Evergreen installé sur Windows est
réutilisé. Discard est un projet indépendant, non affilié à Discord.

## Base actuelle

- Fenêtre native redimensionnable avec prise en charge du DPI.
- Chargement de `https://discord.com/app` et session persistante dans
  `%LOCALAPPDATA%\Discard\WebView2`.
- Boucle Win32 bloquante, initialisation WebView2 asynchrone et fermeture explicite.
- Navigation principale limitée à HTTPS sur `discord.com`, sans bridge natif.
- Nouvelles fenêtres et téléchargements bloqués dans cette première base.
- Formatage, Clippy et compilation Windows dans GitHub Actions.

Cette première version héberge le site officiel. Elle ne fournit pas de serveur
de messagerie indépendant, de client Discord réécrit, d'installation automatique
ou de mise à jour intégrée. Les liens externes sont bloqués pour le moment.
Les ressources et sous-frames du site sont gérées par WebView2.

## Prérequis

- Windows 10/11 x64 et [WebView2 Evergreen Runtime](https://developer.microsoft.com/microsoft-edge/webview2/).
- [Rust stable](https://rustup.rs/) avec `rustfmt` et `clippy`.
- Pour la cible MSVC recommandée : Visual Studio Build Tools, charge de travail
  **Desktop development with C++**, et Windows SDK.

```powershell
cargo run
```

Fermer la fenêtre quitte l'application. Les données de session restent sur disque.
Les autorisations du site suivent les mécanismes de WebView2 ; voix, caméra,
notifications et partage d'écran nécessitent une validation réelle sur Windows.
Aucun gain mémoire ou CPU n'est encore mesuré.

## Développement

```powershell
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo build --release --locked
# Ou :
.\scripts\dev.ps1 check
.\scripts\dev.ps1 build
```

Le binaire MSVC est `target\release\discard.exe`. Avec la cible GNU, livrer aussi
`WebView2Loader.dll` à côté du binaire ; `scripts/dev.ps1 build` le copie.
Les tests unitaires/fonctionnels sont exécutés uniquement sur demande explicite.

Les règles de contribution sont dans [AGENTS.md](AGENTS.md), l'organisation dans
[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), et les outils/skills dans
[docs/TOOLING.md](docs/TOOLING.md).
