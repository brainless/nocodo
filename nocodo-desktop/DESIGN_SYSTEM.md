# NocoDo Desktop Design System

This document outlines the standardized sizing and styling tokens for consistent UI across the desktop app.

## Usage

Import the design system in your `.slint` files:

```slint
import { DesktopSizes, DesktopTypography, DesktopColors } from "theme.slint";
```

## Widget Sizing Reference

### Text Inputs
```slint
// Standard input field
width: DesktopSizes.input_default_width;     // 300px
min-width: DesktopSizes.input_min_width;     // 200px
max-width: DesktopSizes.input_max_width;     // 600px
height: DesktopSizes.input_height;           // 40px
```

### Text Areas
```slint
// Standard textarea
width: DesktopSizes.textarea_default_width;  // 500px
min-width: DesktopSizes.textarea_min_width;  // 300px
max-width: DesktopSizes.textarea_max_width;  // 800px
min-height: DesktopSizes.textarea_min_height; // 120px
```

### Cards
```slint
// Information cards
width: DesktopSizes.card_default_width;      // 360px
min-width: DesktopSizes.card_min_width;      // 280px
max-width: DesktopSizes.card_max_width;      // 480px
```

### Dialogs
```slint
// Dialog sizes
width: DesktopSizes.dialog_small_width;      // 400px
width: DesktopSizes.dialog_medium_width;     // 600px
width: DesktopSizes.dialog_large_width;      // 800px
```

### Buttons
```slint
height: DesktopSizes.button_height;          // 36px
height: DesktopSizes.button_large_height;    // 44px
```

## Spacing Scale (8px Grid)

```slint
spacing: DesktopSizes.space_xs;    // 4px  - minimal spacing
spacing: DesktopSizes.space_sm;    // 8px  - tight spacing
spacing: DesktopSizes.space_md;    // 16px - standard spacing
spacing: DesktopSizes.space_lg;    // 24px - comfortable spacing
spacing: DesktopSizes.space_xl;    // 32px - generous spacing
spacing: DesktopSizes.space_2xl;   // 48px - section spacing
spacing: DesktopSizes.space_3xl;   // 64px - large section spacing
```

## Padding Presets

```slint
padding: DesktopSizes.padding_input;    // 12px - for input fields
padding: DesktopSizes.padding_card;     // 16px - for cards
padding: DesktopSizes.padding_dialog;   // 24px - for dialogs
padding: DesktopSizes.padding_section;  // 32px - for page sections
```

## Border Radius

```slint
border-radius: DesktopSizes.radius_sm;  // 4px  - subtle rounding
border-radius: DesktopSizes.radius_md;  // 8px  - standard rounding
border-radius: DesktopSizes.radius_lg;  // 12px - prominent rounding
border-radius: DesktopSizes.radius_xl;  // 16px - heavy rounding
```

## Typography

### Font Sizes
```slint
font-size: DesktopTypography.font_xs;    // 11px - minimal text
font-size: DesktopTypography.font_sm;    // 12px - small text
font-size: DesktopTypography.font_base;  // 14px - body text
font-size: DesktopTypography.font_md;    // 16px - medium text
font-size: DesktopTypography.font_lg;    // 18px - large text
font-size: DesktopTypography.font_xl;    // 20px - extra large
font-size: DesktopTypography.font_2xl;   // 24px - headings
font-size: DesktopTypography.font_3xl;   // 30px - large headings
```

### Font Weights
```slint
font-weight: DesktopTypography.weight_regular;   // 400
font-weight: DesktopTypography.weight_medium;    // 500
font-weight: DesktopTypography.weight_semibold;  // 600
font-weight: DesktopTypography.weight_bold;      // 700
```

## Colors

### Background Colors
```slint
background: DesktopColors.background;        // #ecf0f1 - page background
background: DesktopColors.surface;           // #ffffff - cards, inputs
background: DesktopColors.surface_variant;   // #f8f9fa - alternate surface
```

### Text Colors
```slint
color: DesktopColors.text_primary;    // #2c3e50 - primary text
color: DesktopColors.text_secondary;  // #7f8c8d - secondary text
color: DesktopColors.text_on_dark;    // #ecf0f1 - text on dark bg
color: DesktopColors.text_muted;      // #95a5a6 - subtle text
```

### Border Colors
```slint
border-color: DesktopColors.border_light;   // #e0e0e0 - subtle borders
border-color: DesktopColors.border_medium;  // #bdc3c7 - standard borders
border-color: DesktopColors.border_dark;    // #95a5a6 - prominent borders
```

### Interactive Colors
```slint
background: DesktopColors.primary;        // #3498db - primary action
background: DesktopColors.primary_hover;  // #2980b9 - hover state
background: DesktopColors.primary_active; // #21618c - active state
```

### Status Colors
```slint
background: DesktopColors.success;  // #27ae60 - success state
background: DesktopColors.warning;  // #f39c12 - warning state
background: DesktopColors.error;    // #e74c3c - error state
background: DesktopColors.info;     // #3498db - info state
```

## Content Width Constraints

For readable content areas:

```slint
max-width: DesktopSizes.content_narrow;  // 600px  - forms, narrow content
max-width: DesktopSizes.content_medium;  // 800px  - standard content
max-width: DesktopSizes.content_wide;    // 1200px - wide content
max-width: DesktopSizes.content_full;    // 1600px - full-width content
```

## Examples

See `component-examples.slint` for practical examples of:
- StandardInput
- StandardTextArea
- InfoCard
- StandardDialog
- StandardForm
- SectionHeader
- ContentContainer

## Best Practices

1. **Always use tokens instead of hardcoded values**
   - ❌ `padding: 16px`
   - ✅ `padding: DesktopSizes.space_md`

2. **Use semantic naming**
   - Use `padding_card` for card components
   - Use `padding_dialog` for dialogs
   - Use `padding_input` for input fields

3. **Follow the 8px grid**
   - All spacing should align to the 8px grid
   - Use the `space_*` tokens which are multiples of 4 or 8

4. **Respect min/max constraints**
   - Set `min-width` and `max-width` for inputs and text areas
   - This ensures good UX across different window sizes

5. **Use appropriate font weights**
   - Headers: `weight_semibold` or `weight_bold`
   - Body text: `weight_regular`
   - Emphasized text: `weight_medium`

## Customization

To customize the design system, edit `nocodo-desktop/ui/theme.slint`. All components using the tokens will automatically update.
