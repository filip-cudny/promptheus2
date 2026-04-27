# Bug: przełączenie na Claude w istniejącym oknie nie działa

## Objaw

W istniejącym oknie conversation-dialog:
1. User otwiera dialog (Promptheus widoczny).
2. Wybiera providera ChatGPT (kolokwialnie nazwany „OpenAI") z palety / dropdownu.
3. Pisze parę wiadomości w ChatGPT.
4. Otwiera paletę i wybiera Claude → **nic się nie dzieje wizualnie**.
5. Może wrócić do Promptheusa lub ChatGPT.
6. „Open in new window" + Claude → działa poprawnie.

Default providerzy: tylko `claude` i `chatgpt` (`src-tauri/src/models/settings.rs:104`). „OpenAI" w opisie usera = ChatGPT.

## Architektura przełączania (oba ścieżki zbiegają się w jednym miejscu)

- **Paleta** (Ctrl+P + Enter / klik): `ConversationDialogApp.dismissPalette` → `closePalette` (`src/lib/services/shellToolbar.ts:23`) → komenda `close_palette` (`src-tauri/src/commands/ai_webview.rs:163`) → `swap_from_palette` (`src-tauri/src/services/ai_webview.rs:584`) → `hosted_swap_to_provider`.
- **Toolbar / dropdown**: `ShellToolbarApp.selectProvider` (`src/ShellToolbarApp.svelte:69`) → `swap_ai_webview` → `swap_to_provider` → `hosted_swap_to_provider`.

`hosted_swap_to_provider` (`src-tauri/src/services/ai_webview.rs:330`):
- `already_created = state.has_hosted_child(host_label, provider_id)`
- jeśli `false`: tworzy nowy child webview przez `host.add_child(...)` — **ciężka ścieżka, tylko tutaj jest tworzenie**
- show target, hide reszty (poza toolbarem)

## Hipoteza

Promptheus i ChatGPT (już utworzony) chodzą tylko przez show/hide. Claude — pierwszy raz w tym hoście — wymaga `add_child` na hoście, który już ma czworo dzieci (`shell-toolbar`, `conversation-dialog`, `conversation-dialog::ai-webview-chatgpt`). To jedyny scenariusz wymagający dodania **kolejnego** ai-webview do żywego hosta.

W nowym oknie `add_child` dla Claude robi się na świeżym hoście z dwójką dzieci (`shell-toolbar-N` + `conversation-dialog-N`) — działa.

Pasuje do **Ryzyka #4** z `.todo/plans/0037-ai-webview-single-window-multichild.md`: „Linux WebKitGTK multi-webview — webkit2gtk ma mniej testów dla wielu WebView w jednym GtkWindow. Możliwe artefakty visual / focus."

Każde child webview na Linux X11 (`wry-0.54.4/src/webkitgtk/mod.rs:140` `new_x11`) tworzy osobne X11 window via `XCreateSimpleWindow` reparentowane do hosta, owijane w osobny GTK Toplevel z własnym vboxem. `webview.show()`/`hide()` to `XMapWindow`/`XUnmapWindow` + `gtk_window.show_all()`/`hide()`.

## Dodane logi (commit pending)

Plik: `src-tauri/src/services/ai_webview.rs`

1. **`hosted_swap_to_provider`**: ENTER, lista pre-state webviews, decyzja `already_created`, parametry `add_child` (pos/size/host_size), wynik `add_child` (OK/FAILED z błędem), lista post-create webviews, wynik `target.show()`, każdy `hide()` (OK/FAILED), `set_focus`.
2. **`swap_to_palette`**: ENTER, `previous_active`, no-op gdy paleta już otwarta.
3. **`swap_from_palette`**: ENTER, `previous_active`, lookup providera, decyzja routingu (Promptheus / provider / restore).
4. **`on_navigation`** w child webview: log gdy router sentinel wykryty (potwierdza, że Ctrl+P z ChatGPT dotarł do Rust).
5. **`on_page_load`** w child webview: log gdy `Finished` z URL-em (potwierdza, że claude.ai się załadował).

`cargo check` przechodzi.

## Jak zebrać logi

```bash
cd /home/filip/ai-tools/promptheus-migrate/promptheus-tauri && \
RUST_LOG=app_lib::services::ai_webview=trace,app_lib::commands::ai_webview=debug \
pnpm tauri dev 2>&1 | tee /tmp/promptheus-claude-debug.log
```

**Reprodukcja:**
1. Otwórz dialog (Promptheus widoczny).
2. Ctrl+P → wybierz ChatGPT.
3. Pisz parę wiadomości w ChatGPT.
4. Ctrl+P z poziomu webview ChatGPT → wybierz Claude.
5. Zostaw aplikację → log w `/tmp/promptheus-claude-debug.log`.

## Drzewo decyzyjne po zebraniu logów

| Co widać | Diagnoza | Następny krok |
|---|---|---|
| Brak `on_navigation: router sentinel detected` po Ctrl+P z ChatGPT | `PALETTE_KEYBIND_JS` nie zainstalował się w ChatGPT (CSP, listener nadpisany przez stronę, on_page_load nie wystrzelił) | Sprawdzić CSP claude.ai/chatgpt.com, ewentualnie inny mechanizm dostarczania keybindu (np. Tauri global shortcut zamiast injected JS) |
| `swap_from_palette: looking up provider id=claude` → `palette: unknown provider claude` | Provider claude wypadł z settings (race / corruption) | Sprawdzić AppState |
| `add_child FAILED` z konkretnym błędem | Logiczny błąd Tauri/wry | Tekst błędu wskaże kierunek |
| `add_child OK` ale brak `on_page_load Finished` dla claude webview | claude.ai się nie ładuje w child webview (możliwy CSP/X-Frame-Options/sandboxing) | Sprawdzić w devtoolsach claude webview, ewentualnie network |
| `add_child OK`, `target.show OK`, `hide OK` dla ChatGPT, `set_focus OK`, `on_page_load Finished` JEST — a wizualnie nic nie widać | **Potwierdzone Ryzyko #4** — logicznie OK, problem na warstwie WebKitGTK / X11 stacking / mapping | Patrz „Możliwe rozwiązania" niżej |
| `hide FAILED label=...ai-webview-chatgpt` | ChatGPT zostaje mapped na wierzchu, zasłaniając Claude | Zbadać dlaczego hide failuje, ewentualnie dodać `webview.set_position` poza ekran jako backup |

## Możliwe rozwiązania (po potwierdzeniu Ryzyka #4)

1. **Hide ChatGPT przed `add_child` Claude.** Dziś jest odwrotnie — najpierw create+show Claude, potem hide reszty. Może to wystarczy do uporządkowania X11 stacking.
2. **Tworzyć child z `attributes.visible = false`, show'ować dopiero po `on_page_load Finished`.** Mitigacja zaproponowana w planie 0037 punkt #2 (white flash) — może też naprawić mapping.
3. **Eager-create wszystkich child webview przy tworzeniu hosta.** Zamiast lazy, robić `add_child` dla wszystkich providerów przy `dialog::open_or_focus`. Eliminuje scenariusz „add_child na żywym hoście z innym dzieckiem".
4. **`cfg(target_os = "linux")` fallback do close+open** (path standalone) dla scenariuszy multi-child — punkt awaryjny z planu 0037 Ryzyko #4.
5. **`webview.reparent` / `gtk_window.present`** — ręczne raise X11 window dla nowo utworzonego child.

## Kluczowe pliki

- `src-tauri/src/services/ai_webview.rs` (ścieżka backendu, dodane logi)
- `src-tauri/src/commands/ai_webview.rs` (komendy IPC: `swap_ai_webview`, `close_palette`, `open_palette`)
- `src-tauri/src/services/dialog.rs` (`open_or_focus`, `apply_layout`, `configure_linux_child_packing`)
- `src/ConversationDialogApp.svelte` (paleta UI)
- `src/ShellToolbarApp.svelte` (toolbar dropdown)
- `.todo/plans/0037-ai-webview-single-window-multichild.md` (architektura + Ryzyka #4 i #6)
- Tauri/wry source: `~/.cargo/registry/src/index.crates.io-*/wry-0.54.4/src/webkitgtk/mod.rs` (X11 child window creation)

## Stan przed kontynuacją

- Logi dodane, kompiluje się (`cargo check` 0 błędów, 38 warnings — pre-existing).
- Brak commitu — zmiany są w worktree.
- Czeka na log z reprodukcji, potem decyzja którą gałąź drzewa decyzyjnego pociągnąć.
