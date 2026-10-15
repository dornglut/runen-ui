# RunenUI Gallery — native host integration in progress

The Gallery will be one independent public-contract RunenUI application where
developers can navigate and interact with accepted M11 controls.

This initial application-model slice exposes **Basics** (Button, disabled
Button) and **Inputs** (Checkbox, TextField) with application-owned state and
ordinary public `UiApp`/M10/M11 contracts. The Gallery uses a selected-page
navigation rail, a distinct live-preview panel and a read-only state inspector.
The rail and preview sit side-by-side at ordinary desktop widths, while narrower
windows wrap the workspace and inspector vertically. Layout tests check that
reflow geometry and control mounting persist, but are not a visual native OS proof. A native winit/wgpu/AccessKit
executable now integrates the same public Gallery application using the
accepted native mapping, keyboard/pointer and M10 text/IME translation edges.
Hosted headless validation does **not** establish actual OS window or IME behavior;
the native walkthrough and the remaining controls are **not complete yet**.

```sh
cargo run -p gallery
cargo run -p gallery --bin gallery
cargo test -p gallery
```

The default run target is `gallery-native`, a real native window host; the
separate `gallery` binary prints a deterministic headless layout representation.
Neither hosted compile/real-wgpu evidence nor that debug dump substitutes for a
physical native keyboard/mouse/IME, focus and resize walkthrough or for M11's
remaining catalog obligations. If the environment has no graphical display,
the native binary may be unable to open a window.
## Required native acceptance walkthrough (not yet observed)

Run the native target on a graphical desktop with a working GPU. Record the OS,
display protocol, DPI/scale, input method and renderer used; distinguish each
directly observed result from a compile-only or simulated result.

1. Confirm the **native window opens** and shows Basics, navigation, preview and
   the live inspector. The disabled Button must never activate.
2. Mouse-click **Activate** once: the application-owned count and inspector must
   increase exactly once. Repeat with the keyboard after using **Tab**; observe
   a visible focus outline, Enter/Space activation and stable navigation focus.
3. Navigate to **Inputs**; toggle Checkbox with mouse and keyboard. The UI and
   inspector must agree after each action, without duplicate activations.
4. Focus TextField and enter non-ASCII text (for example, `Hé🙂`). Exercise
   caret movement, selection, backspace/delete, undo and redo; verify the
   application-owned value, revision and selection observations stay consistent.
5. Use an actual platform IME to preedit, commit and cancel; check that preedit
   does not duplicate committed text. Switch pages, change window focus and
   return while an IME session is active; a stale generation must not commit.
6. Resize the window narrow/wide and test display-scale changes when available.
   Navigation and input preview must remain reachable; pointer clicks after
   resize must target the *displayed* surface rather than stale coordinates.
7. Exercise window occlusion, focus loss/return and close. Record any renderer,
   accessibility or input diagnostic. Do not classify untested platforms as
   passing merely because hosted CI and real-wgpu pass.

An observed native walkthrough, independent final review, exact-head
validation and separately owner-authorized protected acceptance are required
before the initial native Gallery slice can be accepted. The broader interactive
control catalog remains a separately tracked future slice.

Counter remains the minimal teaching example and reference_winit remains the
specialized native integration host.
