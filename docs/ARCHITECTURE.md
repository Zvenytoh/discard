# Architecture

## Périmètre

Discard est un exécutable Windows qui héberge Discord Web. Le serveur, les comptes,
les messages et les médias restent fournis par Discord. Aucun backend local,
framework frontend ou appel direct à une API privée Discord n'est ajouté.

```text
src/main.rs          Point d'entrée et erreurs fatales
src/navigation.rs    Politique pure d'URL HTTPS
src/native/mod.rs    STA COM, état, fenêtre, messages Win32 et fermeture
src/native/webview.rs Initialisation WebView2 et événements de navigation
```

## Propriété et durée de vie

Un seul thread UI possède la fenêtre et toutes les interfaces COM. Une garde
équilibre `CoInitializeEx` et `CoUninitialize`. `Rc`, `RefCell` et le stockage
thread-local ne traversent aucun thread. Les références sont clonées et les
emprunts terminés avant les appels natifs susceptibles de réentrer.

Les callbacks récupèrent l'état courant ; une création de contrôleur terminée
après la destruction de la fenêtre ferme immédiatement le contrôleur reçu.
`WM_DESTROY` retire l'état global avant de fermer WebView2 et poste `WM_QUIT`.
Les erreurs asynchrones sont différées par un message Win32 ; aucun dialogue
modal n'est ouvert dans un callback COM.

## Frontière de confiance

La navigation principale accepte uniquement `https://discord.com` sur le port
443, sans identifiants dans l'URL. Le parseur `url` évite les comparaisons de
préfixes et les hôtes trompeurs. Les sous-ressources et iframes conservent les
règles du moteur. Les nouvelles fenêtres et téléchargements sont annulés.
Les objets hôtes et messages WebView2 sont désactivés. Aucun script n'est injecté.
Les autorisations caméra/micro restent soumises aux mécanismes par défaut du moteur.

Le profil se trouve exclusivement dans `%LOCALAPPDATA%\Discard\WebView2` et
n'est jamais versionné. La fermeture ne supprime aucune donnée utilisateur.

## Évolutions

Ajouter séparément les liens externes avec geste utilisateur, les téléchargements
confirmés, la zone de notification et les fonctionnalités média après validation
de leur besoin. Toute évolution de l'allowlist doit citer le flux qui la nécessite.
La base ne comporte ni installateur ni mécanisme de mise à jour.
