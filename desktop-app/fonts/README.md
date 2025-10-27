# Custom Fonts

This directory contains custom fonts that are bundled into the nocodo desktop app binary.

## Current Fonts

### UI Widget Fonts (Ubuntu Sans)

**Ubuntu Sans** - Canonical's signature font for UI elements
- **UbuntuSans-Light.ttf** (474 KB) - Used for regular UI widgets (labels, status messages, navigation)
- **UbuntuSans-SemiBold.ttf** (476 KB) - Used for emphasis (buttons, headings, CTAs)

### User Content Fonts (Inter)

**Inter** - A typeface carefully crafted & designed for computer screens
- **Inter-Regular.ttf** (402 KB) - Used for all user-generated content, titles, file contents
- **Inter-Medium.ttf** (408 KB) - Used for code blocks and monospace text
- **Inter-SemiBold.ttf** (410 KB) - Available for future use

## Licenses

**Ubuntu Sans**: Ubuntu Font License (UFL)
- Source: https://github.com/canonical/Ubuntu-fonts
- License: https://ubuntu.com/legal/font-licence

**Inter**: SIL Open Font License 1.1
- Source: https://rsms.me/inter/
- License: https://github.com/rsms/inter/blob/master/LICENSE.txt

## How Fonts are Used

Fonts are registered globally in `src/app.rs` using the `setup_fonts()` function:
- `FontFamily::Proportional` → Inter Regular (default for all text)
- `FontFamily::Monospace` → Inter Medium (for code)

The fonts are embedded at compile time using `include_bytes!()` macro, so they become part of the binary and don't need to be installed on the user's system.

## Adding New Fonts

1. Download `.ttf` or `.otf` files and place them in this directory
2. Update `src/app.rs` in the `setup_fonts()` function:
   ```rust
   const MY_FONT: &[u8] = include_bytes!("../fonts/MyFont.ttf");
   fonts.font_data.insert(
       "my_font".to_owned(),
       std::sync::Arc::new(egui::FontData::from_static(MY_FONT)),
   );
   ```
3. Add to appropriate font family or create a custom one
4. Rebuild the app

## Changing Fonts

To change the default fonts across the entire app, simply modify the font priorities in `setup_fonts()` in `src/app.rs`.
