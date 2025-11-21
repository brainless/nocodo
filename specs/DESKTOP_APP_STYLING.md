# Desktop App Styling Specifications

This document contains styling specifications and patterns for consistent UI design across the nocodo desktop application.

## Modal/Form Styling

### Reference Implementation
The "Connect to Server" modal (shown when clicking "+ New Server" button) serves as the reference implementation for modal forms.

**Location:** `desktop-app/src/components/connection_dialog.rs`

### Specifications

#### Window Properties
- **Fixed Width:** `320.0` pixels
- **Height:** Auto-sizing (set to `0.0` for automatic height calculation)
- **Collapsible:** `false`
- **Resizable:** `false`

```rust
egui::Window::new("Title")
    .collapsible(false)
    .resizable(false)
    .fixed_size(egui::vec2(320.0, 0.0))
```

#### Inner Frame Margin
- **Type:** Uniform margin on all sides
- **Value:** `4.0` pixels

```rust
egui::Frame::NONE
    .inner_margin(egui::Margin::same(4))
```

#### Form Field Spacing
- **Vertical spacing between fields:** `4.0` pixels (using `ui.add_space(4.0)`)
- **Layout:** Each field consists of:
  1. Label (e.g., "SSH Server:")
  2. Input widget (immediately below label)
  3. Vertical space before next field

```rust
ui.label("Field Label:");
ui.add(
    egui::TextEdit::singleline(&mut field_value)
        .desired_width(f32::INFINITY)
);
ui.add_space(4.0);
```

#### Text Input Widgets
- **Width:** Full available width (`f32::INFINITY`)
- **Type:** Single-line text edit for most fields

```rust
egui::TextEdit::singleline(&mut value)
    .desired_width(f32::INFINITY)
```

#### Sections and Separators
- **Section Spacing:** `10.0` pixels before sections (e.g., before "Your SSH Public Key" section)
- **Horizontal Separator:** Used to divide major sections

```rust
ui.add_space(10.0);
ui.separator();
```

#### CTA (Call-to-Action) Buttons
- **Location:** Bottom of modal
- **Separator:** Horizontal line (`ui.separator()`) before button row
- **Spacing after separator:** `4.0` pixels
- **Layout:** Horizontal row
- **Button Padding:** `egui::vec2(6.0, 4.0)` (6px horizontal, 4px vertical)

```rust
ui.separator();
ui.add_space(4.0);

ui.horizontal(|ui| {
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = egui::vec2(6.0, 4.0);

        if ui.button("Primary Action").clicked() {
            // action
        }

        if ui.button("Cancel").clicked() {
            // cancel
        }
    });
});
```

### Applied To

These specifications have been applied to:
- "+ New Server" modal in Servers page (`desktop-app/src/components/connection_dialog.rs`)
- "Connect" form in Servers page (`desktop-app/src/components/connection_dialog.rs`)
- Register/Login forms (`desktop-app/src/components/auth_dialog.rs`)
