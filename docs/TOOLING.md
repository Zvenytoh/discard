# Outils et skills

## Répartition

| Outil | Portée | Usage |
| --- | --- | --- |
| AGENTS.md | Dépôt | Règles du projet, toujours lues |
| RTK | Global | Sorties compactes Git/Cargo, commandes prises en charge |
| Ponytail | Skill global | Implémentations simples et minimales |
| Superpowers | Skills globaux | Planification complexe et diagnostic, invocation ciblée |
| Graphify | `.agents/skills/graphify` | Questions d'architecture si un index existe |
| ast-grep | Global | Recherche structurelle/refactoring Rust |
| Context7 | MCP global | Documentation récente uniquement si nécessaire |
| Semgrep | Global | Audit ponctuel de sécurité |
| OpenAI Skills | Globaux | Sélection selon le besoin ; pas de copie de tout le catalogue |

Les binaires utilisateur sont installés dans `%USERPROFILE%\.local\bin`.
Ouvrir un nouveau terminal après modification du PATH. Les skills globaux
Ponytail et les skills OpenAI spécialisés déjà présents sont réutilisés.
Superpowers est disponible via `%USERPROFILE%\.agents\skills\superpowers`,
une jonction vers `%USERPROFILE%\.codex\superpowers\skills`.
Ses mises à jour se font explicitement avec `git -C "$env:USERPROFILE\.codex\superpowers" pull --ff-only`.
Les instructions utilisateur priment sur les workflows des skills, notamment
pour les tests et les sous-agents.

## Exemples

```powershell
rtk git status --short
rtk cargo build --release --locked
ast-grep run --lang rust --pattern 'unsafe { $$$BODY }' src
graphify query "Comment la fenêtre possède-t-elle le contrôleur WebView2 ?"
semgrep scan --config p/rust --metrics=off --error src
```

Graphify est installé mais aucun index n'est créé pour ce petit socle. Ne lancer
la génération qu'en cas de besoin ; `graphify-out/` est ignoré par Git. Le skill
Graphify inclut sa licence et ses références dans le dépôt.

Context7 est déclaré dans `%USERPROFILE%\.codex\config.toml` avec l'endpoint
`https://mcp.context7.com/mcp`. Si le service demande une authentification ou si
le quota anonyme est épuisé, connecter le compte depuis les réglages MCP de Codex.
Ne jamais ajouter de clé API au dépôt. Les nouveaux skills sont chargés dans une
nouvelle conversation/session Codex.

## Environnement de cette machine

Un fichier local ignoré `.tools/env.ps1` réutilise la chaîne Rust/GNU déjà présente
sur cette machine. Il dépend de son emplacement actuel et modifie uniquement
l'environnement du terminal courant. Depuis le dossier Discard :

```powershell
. .\.tools\env.ps1
.\scripts\dev.ps1 check
.\scripts\dev.ps1 build
.\scripts\dev.ps1 run
```

Pour une autre machine, installer les prérequis MSVC du README. La CI utilise
MSVC ; le contrôle local initial utilise GNU, faute de linker Visual C++ installé.

## Sources

- [RTK](https://github.com/rtk-ai/rtk)
- [Superpowers](https://github.com/obra/superpowers)
- [ast-grep](https://github.com/ast-grep/ast-grep)
- [Context7 pour Codex](https://github.com/upstash/context7/blob/master/docs/clients/codex.mdx)
- [Configuration MCP officielle de Codex](https://developers.openai.com/codex/mcp)
- [Semgrep](https://semgrep.dev/docs/getting-started/)
