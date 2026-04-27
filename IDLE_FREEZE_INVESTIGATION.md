# Idle freeze: Conversation Dialog z ChatGPT WebUI (Linux/WebKitGTK)

Notatka diagnostyczna do błędu, w którym po dłuższej bezczynności okno
Conversation Dialog (host z hostowanym providerem ChatGPT) przestaje
odpowiadać — context menu nie odpala się, hotkeye nie skutkują, system
pokazuje "Wait / Force Quit".

Stan: **niereprodukowane w kontrolowanych warunkach**, czekamy na powtórzenie
z włączonymi logami diagnostycznymi (sekcja niżej).

---

## Reprodukcja

1. Otworzyć Conversation Dialog (host).
2. Przełączyć się na providera ChatGPT WebUI (palette → ChatGPT).
3. Zostawić w tle przez **kilkanaście–kilkadziesiąt minut**, pracując z
   innymi aplikacjami (monitor może iść w sleep).
4. Próbować wywołać context menu (Ctrl+F1) lub kliknąć w okno.

Oczekiwane: menu się pojawia, dialog odpowiada.
Obserwowane: brak reakcji, system menedżer okien pokazuje "wait/force quit".

## Symptomy z logów produkcyjnych (przykład 2026-04-27 15:20)

```
15:01:42 …                                               ← ostatnia aktywność
… (cisza ~19 min) …
15:20:40 hotkey action: control+F1 -> open_context_menu
15:20:40 show context menu: cursor=(1154, 984), work_area=(0, 360, 1920x1080)
         ← brak "focus_context_menu: present_with_time(...)"
15:20:42 hotkey action: control+F1 -> open_context_menu  ← nawet "show context menu" już nie loguje
15:20:43 hotkey action: control+F1 -> open_context_menu
15:20:45 hotkey action: control+F1 -> open_context_menu
15:21:21 config loaded                                   ← restart aplikacji
```

Charakterystyczne:
- Hotkey-handler dalej się odpala (osobny wątek pluginu `tauri-plugin-global-shortcut`),
  ale jego akcje nie dochodzą do końca.
- Pierwsza próba dochodzi do `show context menu` (logi w `show_context_menu_window`
  do `cursor_position()` włącznie). Następnie `focus_context_menu` nigdy się nie
  loguje. Kolejne próby nie logują nawet `show context menu`.
- Gap aktywności 19 min — silny sygnał, że to problem **idle**, nie obciążeniowy.

## Diagnoza (przyczyna źródłowa)

Wątek główny GTK / pętla GLib procesu Promptheus zostaje zablokowany, bo
**content process WebKit dla ChatGPT staje się nieresponsywny** w idle.

Architektura: Tauri 2 z `unstable` feature pozwala hostować ChatGPT jako
webview-dziecko okna `conversation-dialog` (`add_child`). Dziecko współdzieli
z hostem:

- pętlę główną GTK / GLib,
- synchroniczny IPC X11 z menedżerem okien,
- WebKit `WebKitWebContext` (po stronie UI procesu).

Gdy content process WebKit wisi (lub wisi WebProcess <-> UIProcess IPC),
każda operacja wątku głównego, która oczekuje synchronicznej odpowiedzi,
też wisi. To dotyczy:

- `cursor_position()`, `available_monitors()` (`find_monitor_at`)
- doręczenia IPC do innego webview hostowanego w tym samym oknie
- `gtk_window().present_with_time(...)` poprzez `gdkx11::functions::x11_get_server_time(&x11_win)`
  (synchroniczny round-trip X11)
- `set_focus()` (Tauri robi to samo `present_with_time(GDK_CURRENT_TIME)`)

Hotkey-handler żyje, bo plugin ma własny niezależny wątek. Akcja, którą
dyspatchuje, próbuje dotknąć wątku głównego i tam się zatrzymuje.

### Hipotezy o pochodzeniu zatrzymania content procesu ChatGPT

1. **Utrata kontekstu kompozytora WebKit GPUProcess po sleep monitora** —
   znany problem WebKitGTK na X11/Wayland, content/GPU process traci kontekst
   i nie wraca; UIProcess czeka.
2. **Zacięty reconnect websocketu / SSE** — ChatGPT trzyma long-running
   stream; reset stream-a w idle może wisieć na readzie bez timeoutu.
3. **Cloudflare turnstile / challenge auth** zaserwowany w bezczynnym
   webview w tle — może wpaść w pętlę.
4. **Heap pressure / GC w długo żyjącej stronie React** — niewykluczone,
   ale mniej prawdopodobne niż 1–2 dla scenariusza idle.

Identyfikacja właściwej hipotezy wymaga porównania logów heartbeat z
sygnałem `web-process-terminated` w momencie freeze (patrz: dodane logi).

---

## Plan działań — co było rekomendowane (NIE wdrożone)

Rozważano kombinację `#2 + #1 + #4`:

### #1 — Auto-recovery na `web-process-terminated`

Sygnał WebKitGTK emitowany przy śmierci content procesu. Handler w
`services/ai_webview.rs::install_media_permissions`:

```rust
wk.connect_web_process_terminated(move |wv, reason| {
    log::warn!("web process terminated: reason={reason:?}");
    wv.reload();
});
```

Pomaga **tylko** gdy proces faktycznie umiera. Jeśli wisi w live-locku /
nieskończonym I/O — sygnał się nie pojawia, więc nic to nie da.

### #2 — Suspend ukrytych providerów

W ścieżkach hide w `hosted_swap_to_provider`,
`hosted_swap_to_conversation_dialog`, `swap_to_palette`,
`restore_previous_webview` — zamiast samego `webview.hide()`, dodatkowo
ubić content process providera:

```rust
webview.with_webview(|pv| {
    pv.inner().terminate_web_process();
});
```

Stan: `AiWebviewState.suspended: HashSet<String>` (pełny label webview).
Na pokazanie ukrytego wcześniej providera (`target.show()`) →
`webview.reload()` i `unmark_suspended`.

Handler `web-process-terminated` musi sprawdzać `is_suspended`, by nie
reloadować przy świadomym suspendzie (inaczej nieskończona pętla
terminate→reload).

Plus: ChatGPT/Claude w tle nie pali CPU/sieci/RAM, więc nie ma nawet okazji
do wystąpienia tego błędu na "drugorzędnych" providerach.
Minus: stan sesji panelu (przewinięcie, half-typed message) ginie; cookie
zostają, więc nie ma re-loginu, ale aktywna konwersacja musi się
przeładować.

### #4 — Manual escape: przycisk "Reload provider" w shell-toolbar

**ODRZUCONE** świadomie 2026-04-27. Powód: gdy wątek główny jest zamrożony
z powodu nieresponsywnego content procesu, klik w przycisk toolbara nie
dociera do handlera (event loop GTK leży). Reload button nie pomaga w
faktycznym freeze — pomaga tylko w "miękkich" przypadkach typu strona
mocno spowolniona ale nie zacięta.

### Świadomie odrzucone alternatywy

- **Bump `webkit2gtk` do 2.46+**: pinned przez wry, wymaga upgrade'u Tauri,
  duży blast radius. Nie wiadomo czy nowsze wersje rozwiązują problem.
- **Każdy provider w osobnym oknie**: dalej współdzielą pętlę główną GTK
  procesu — nie izoluje.
- **Osobny `WebKitWebContext` per provider**: izoluje cookies/storage, ale
  nie izoluje wątku głównego ani IPC.
- **`pnpm tauri build` z innym backendem renderingu**: WebKitGTK to jedyna
  realna opcja dla wry na Linuxie.

## Co realnie mogłoby pomóc (do rozważenia po reprodukcji)

- **Watchdog na main thread**: tokio co N s `app.run_on_main_thread(send())`
  z timeoutem; po nieotrzymaniu echa — log warn + opcjonalnie restart
  hosting webview (bez ubijania całego procesu).
- **Async / spawn_blocking** dla `cursor_position()`, `available_monitors()`,
  `x11_get_server_time` — żeby blokowanie się tych callów nie trzymało
  tokio executora.
- **Inactivity timer** per provider child: po X minut idle, automatycznie
  `terminate_web_process()` (= soft-suspend bez ingerencji użytkownika).
- **Migracja na xdg-portal / Wayland-native focus** — eliminuje X11
  round-trip w `x11_get_server_time`.

---

## Co zostało zrobione w tej sesji

Cofnięto wszystkie zmiany behawioralne (`#1` auto-reload, `#2` suspend-on-hide,
`#4` reload button). Pozostawiono **wyłącznie logi diagnostyczne**, których
celem jest jednoznaczne wskazanie miejsca zacięcia przy następnej
reprodukcji.

### A) `services/ai_webview.rs::install_media_permissions`

Refaktor: funkcja przyjmuje teraz `webview_label: String`. Dodano
**obserwacyjny** handler `connect_web_process_terminated` który tylko
loguje:

```
web process terminated: webview=<label> reason=<…>
```

**Bez** reloadu. Cel: stwierdzić w logach czy content process w ogóle
umiera (sygnał się pojawia) czy tylko wisi (cisza w logach).

### B) `lib.rs` setup — heartbeat wątku głównego

Tokio task co 5 s:

```
heartbeat dispatch #N           ← z tokio runtime
heartbeat tick #N delay=Xms     ← z GTK main thread (po app.run_on_main_thread)
```

Interpretacja:
- Healthy: `dispatch` i `tick` przeplatane, delay < 100 ms.
- Freeze: `dispatch` rośnie, `tick` zatrzymuje się.
  Numer ostatniego `tick` + numer pierwszego `dispatch` po wznowieniu →
  dokładnie ile trwał freeze.
- Tokio runtime żyje niezależnie od GTK main loop, więc heartbeat działa
  nawet gdy reszta jest martwa.

### C) `commands/menu.rs`

W `show_context_menu_window` i `focus_context_menu` — per-step elapsed
logi z `std::time::Instant`:

- `show_context_menu_window`: `cursor_position`, `find_monitor_at`, `emit_to`
- `focus_context_menu`: `gtk_window`, `x11_get_server_time`, `present_with_time`

Cel: po kolejnym freeze widać dokładnie który call zaciął się i jak długo.

### D) `services/dialog.rs::focus_window` / `focus_host_window`

Te same per-step elapsed logi wokół `gtk_window`, `x11_get_server_time`,
`present_with_time`. Te ścieżki są wykorzystywane przez focus dialogu
i mogą być równie podatne na zacięcie.

### Lokalizacja logów

```
~/.local/share/com.promptheus.desktop/logs/Promptheus.log
```

Wszystkie logi diagnostyczne są na poziomie `debug`/`info` — żadnego
specjalnego `RUST_LOG` nie trzeba (`app_lib` ma domyślnie `debug`).

---

## Procedura analizy po następnej reprodukcji

1. Odtwórz scenariusz: ChatGPT w hostowanym dialogu, idle 15+ min, próba
   wywołania menu.
2. Otwórz log z `~/.local/share/com.promptheus.desktop/logs/Promptheus.log`.
3. Znajdź ostatni `heartbeat tick #N` przed problemem; porównaj z
   `heartbeat dispatch #M` po — różnica `M - N` × 5 s = czas freeze.
4. Sprawdź czy w okolicy występuje `web process terminated: webview=…`:
   - **Pojawia się** → content process umarł, hipoteza #1 (crash GPUProcess
     albo crash content procesu). Implementuj `#1` (auto-reload na sygnale).
   - **Nie pojawia się** → process wisi, nie umiera. Implementuj `#2`
     (suspend-on-hide z proaktywnym `terminate_web_process()`) lub watchdog
     z timeoutem.
5. W per-step logach `commands/menu.rs` / `services/dialog.rs` znajdź
   pierwszy log bez następnika — ten konkretny call jest miejscem zacięcia.
   Realne kandydaty: `x11_get_server_time` (X11 round-trip), `gtk_window()`
   (czeka na main loop), `emit_to(...)` (IPC do dziecka).

---

## Kontekst technologiczny

- Tauri `=2.10.3`, feature `unstable` (wymagane dla `add_child` /
  multi-webview)
- `webkit2gtk =2.0.2` (Rust crate, pinned)
- `gtk 0.18`, `gdkx11 0.18`
- Linux/X11, GNOME (xdotool dla `detect_frontmost_app_x11`)

## Pliki, których to dotyczy

- `src-tauri/src/services/ai_webview.rs` — hosting multi-webview,
  `install_media_permissions`, swap functions
- `src-tauri/src/services/dialog.rs` — `focus_window`, `focus_host_window`
- `src-tauri/src/commands/menu.rs` — `show_context_menu_window`,
  `focus_context_menu`
- `src-tauri/src/lib.rs` — heartbeat task w setup, hotkey handler
- `src-tauri/Cargo.toml` — wersje webkit2gtk, tauri, gtk
