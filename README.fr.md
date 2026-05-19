# demos RMUX

Cinq petites demos qui montrent rmux comme backend de terminaux programmable.

Le binaire `rmux` doit etre installe et disponible dans le `PATH`.

## Warning securite

> [!WARNING]
> For testing purposes, some demos start AI CLIs with approval or sandbox bypass flags. Be careful with the commands you run, and only use these demos in directories you trust.

## Demos

Voici quelques demos courtes de ce que vous pouvez construire avec RMUX. RMUX ouvre une nouvelle classe de workflows terminal-native, surtout pour l'*orchestration multi-agents*. Il manque encore une demo : votre projet. Si vous construisez quelque chose avec RMUX, envoyez une pull request pour l'ajouter ici.

<!-- rmux-demo-gallery:start -->
<!-- rmux-demo-gallery-item:start -->
<p>
  <a href="https://github.com/Helvesec/rmux-demos/tree/main/demo-orchestration">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/readme/demo-orchestration-header-dark.svg">
      <img src="assets/readme/demo-orchestration-header.svg" alt="Orchestration Demo" width="650">
    </picture>
  </a><br>
  <sub><em>≃ 514 lines</em></sub><br>
  <a href="https://rmux.io/#demo-orchestration">
    <img src="assets/readme/demo-orchestration-preview.png" alt="Lire la video Orchestration Demo" width="720">
  </a>
</p>
<!-- rmux-demo-gallery-item:end -->

<!-- rmux-demo-gallery-item:start -->
<p>
  <a href="https://github.com/Helvesec/rmux-demos/tree/main/broadcast-demo">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/readme/demo-broadcast-header-dark.svg">
      <img src="assets/readme/demo-broadcast-header.svg" alt="Broadcast Demo" width="650">
    </picture>
  </a><br>
  <sub><em>≃ 2,171 lines</em></sub><br>
  <a href="https://rmux.io/#demo-broadcast">
    <img src="assets/readme/demo-broadcast-preview.png" alt="Lire la video Broadcast Demo" width="720">
  </a>
</p>
<!-- rmux-demo-gallery-item:end -->

<!-- rmux-demo-gallery-item:start -->
<p>
  <a href="https://github.com/Helvesec/rmux-demos/tree/main/mini-zellij">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/readme/demo-zellij-header-dark.svg">
      <img src="assets/readme/demo-zellij-header.svg" alt="Zellij Demo" width="650">
    </picture>
  </a><br>
  <sub><em>≃ 944 lines</em></sub><br>
  <a href="https://rmux.io/#demo-zellij">
    <img src="assets/readme/demo-zellij-preview.png" alt="Lire la video Zellij Demo" width="720">
  </a>
</p>
<!-- rmux-demo-gallery-item:end -->

<!-- rmux-demo-gallery-item:start -->
<p>
  <a href="https://github.com/Helvesec/rmux-demos/tree/main/web-claude-demo">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/readme/demo-mirroring-header-dark.svg">
      <img src="assets/readme/demo-mirroring-header.svg" alt="Mirroring Demo" width="650">
    </picture>
  </a><br>
  <sub><em>≃ 649 lines</em></sub><br>
  <a href="https://rmux.io/#demo-mirroring">
    <img src="assets/readme/demo-mirroring-preview.png" alt="Lire la video Mirroring Demo" width="720">
  </a>
</p>
<!-- rmux-demo-gallery-item:end -->

<!-- rmux-demo-gallery-item:start -->
<p>
  <a href="https://github.com/Helvesec/rmux-demos/tree/main/terminal-playwright-demo">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="assets/readme/demo-playwright-header-dark.svg">
      <img src="assets/readme/demo-playwright-header.svg" alt="Playwright Demo" width="650">
    </picture>
  </a><br>
  <sub><em>≃ 1,495 lines</em></sub><br>
  <a href="https://rmux.io/#demo-playwright">
    <img src="assets/readme/demo-playwright-preview.png" alt="Lire la video Playwright Demo" width="720">
  </a>
</p>
<!-- rmux-demo-gallery-item:end -->
<!-- rmux-demo-gallery:end -->

## Dossiers de demos

- `broadcast-demo`: un prompt lance une course entre plusieurs CLI IA.
- `mini-zellij`: mini workspace type Zellij propulse par rmux.
- `web-claude-demo`: navigateur et terminal attaches au meme pane.
- `demo-orchestration`: Claude controle Codex, Gemini et Grok via rmux.
- `terminal-playwright-demo`: tests Playwright-style pour applications terminal.

## Demos Rust

Depuis le dossier de la demo:

```bash
cargo run -- check
cargo run
cargo run -- cleanup
```

## Demo orchestration

Linux et macOS:

```bash
cd demo-orchestration
./launch.sh check
./launch.sh
./launch.sh cleanup
```

Windows PowerShell:

```powershell
cd demo-orchestration
.\launch.ps1 check
.\launch.ps1
.\launch.ps1 cleanup
```
