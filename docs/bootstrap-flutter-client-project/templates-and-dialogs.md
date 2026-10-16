# Templates and the dialogs scaffold

The twin of the Dioxus client's `templates-and-dialogs` topic.

## Templates

Templates are the page layouts every screen is put into.

```
lib/templates/
├── full_screen_form.dart   ← pre-auth pages: SafeArea + a centred card that scrolls from its top
└── app_shell.dart          ← post-auth pages: the app bar · the screen · the tab bar ·
                              the platform toast · the sheet host (RenderDialog)
```

```dart
// lib/templates/full_screen_form.dart
Widget fullScreenForm(Widget content) => SafeArea(
      child: LayoutBuilder(
        builder: (context, constraints) => SingleChildScrollView(
          child: ConstrainedBox(
            constraints: BoxConstraints(minHeight: constraints.maxHeight),
            // Centred while it fits; scrolls from its top when the keyboard takes the space —
            // the twin of the web's `margin: auto`, never `justify-content: center`
            child: Center(child: content),
          ),
        ),
      ),
    );
```

- **The phone's shell is not a desktop menu panel.** An app bar on top, a tab bar at the bottom
  (above the home indicator), modals as bottom sheets, ≥ 44 px targets — the layout rules are
  `get_flutter_design_patterns`, topic `theme`.
- **Tabs the web serves at one address are state of the shell**, not routes (topic `routing`).

## Dialogs — the initial scaffold

The full dialog lifecycle — opening by state, results through `EventHandler`, `dialogTemplate`, the
sheet host — is `get_flutter_design_patterns` §9. The minimal scaffold:

```dart
// lib/dialogs/dialog_state.dart
sealed class DialogState {
  const DialogState();
}

final class DialogNone extends DialogState {
  const DialogNone();
}
```

```dart
// lib/dialogs/render.dart
class RenderDialog extends StatelessWidget {
  const RenderDialog({super.key});

  @override
  Widget build(BuildContext context) => switch (context.watch<AppState>().dialog) {
        DialogNone() => const SizedBox.shrink(),
      };
}
```

A new dialog is a new `final class` of `DialogState` — and every `switch` over it stops compiling
until it is handled.
