# nocodo Website Design System Analysis

## Color Patterns from Homepage & About Page

### Background Colors
- **page_background**: `bg-gradient-to-br from-slate-950 via-slate-900 to-slate-950`
- **page_background_alt**: `bg-slate-950` (flat, used in some sections)
- **page_background_secondary**: `bg-slate-900` (alternating sections)
- **content_box_bg**: `bg-slate-800`
- **content_box_border**: `border-slate-700`
- **card_bg**: `bg-gradient-to-br from-slate-800 to-slate-900`
- **card_border**: `border-slate-700`
- **card_hover_border**: `border-emerald-500/50` or `border-cyan-500/50`

### Text Colors
- **page_title**: `text-slate-100` (4xl, bold)
- **page_subtitle**: `text-slate-300` (xl or lg)
- **section_heading**: `text-slate-100` (3xl or 4xl, bold)
- **section_subheading**: `text-slate-300` (xl)
- **card_title**: `text-slate-100` (xl or 2xl, bold or semibold)
- **body_text**: `text-slate-300`
- **secondary_text**: `text-slate-400`
- **muted_text**: `text-slate-500`

### Accent Colors
- **brand_gradient**: `bg-gradient-to-r from-emerald-400 to-cyan-400`
- **accent_emerald**: `text-emerald-400`
- **accent_emerald_bg**: `bg-emerald-500/10`
- **accent_emerald_border**: `border-emerald-500/20`
- **accent_cyan**: `text-cyan-400`
- **accent_cyan_bg**: `bg-cyan-500/10`
- **accent_cyan_border**: `border-cyan-500/20`

### Interactive Elements
- **link**: `text-emerald-400 hover:text-emerald-300`
- **button_primary**: `bg-gradient-to-r from-emerald-500 to-cyan-500 hover:from-emerald-600 hover:to-cyan-600 text-white`
- **button_step_number**: `bg-emerald-500 text-white` (for numbered steps)
- **icon_container**: `bg-emerald-500/10` or `bg-cyan-500/10`
- **icon_color**: `text-emerald-400` or `text-cyan-400`
- **checkmark_icon**: `text-emerald-400` or `text-cyan-400`

### Policy Link Cards
- **policy_link_bg**: `border border-slate-600 text-slate-200`
- **policy_link_hover**: `hover:bg-slate-700 hover:border-emerald-500/50`
- **policy_link_icon**: `text-slate-400`

---

## Typography Patterns

### Headings
- **page_title_size**: `text-4xl` (mobile) → could scale up on desktop
- **section_heading_size**: `text-3xl sm:text-4xl`
- **card_heading_size**: `text-xl` or `text-2xl`
- **subheading_size**: `text-lg`

### Weights
- **heading_weight**: `font-bold`
- **subheading_weight**: `font-semibold`
- **body_weight**: normal (default)

### Line Heights & Spacing
- **body_leading**: `leading-relaxed`
- **prose_style**: `prose prose-lg prose-invert max-w-none`

---

## Component Patterns

### 1. Page Header
```
<div class="text-center mb-12 md:mb-16">
  <h1 class="text-4xl font-bold text-slate-100 mb-4">
  <p class="text-xl text-slate-300 max-w-3xl mx-auto">
```

### 2. Section Container
```
<section class="py-20 bg-slate-950">
  <div class="max-w-6xl mx-auto px-4 sm:px-6 lg:px-8">
    <div class="text-center mb-16">
      <h2 class="text-3xl sm:text-4xl font-bold mb-6 text-slate-100">
      <p class="text-xl text-slate-300">
```

### 3. Content Box
```
<div class="bg-slate-800 rounded-lg shadow-md border border-slate-700 p-8 mb-12">
  <div class="prose prose-lg prose-invert max-w-none">
```

### 4. Card Grid Item
```
<div class="bg-slate-800 rounded-xl p-6 border border-slate-700">
  <h3 class="text-lg font-bold text-slate-100 mb-2">
  <p class="text-slate-400">
```

### 5. Feature Card with Icon
```
<div class="bg-gradient-to-br from-slate-800 to-slate-900 rounded-2xl p-8 border border-slate-700 hover:border-emerald-500/50 transition-all duration-300">
  <div class="flex items-center mb-6">
    <div class="w-12 h-12 bg-emerald-500/10 rounded-lg flex items-center justify-center mr-4">
      <svg class="w-6 h-6 text-emerald-400">
    <h3 class="text-2xl font-bold text-slate-100">
  <p class="text-slate-300 mb-6 leading-relaxed">
```

### 6. Numbered Step
```
<div class="flex items-start">
  <div class="w-10 h-10 bg-emerald-500 text-white rounded-lg flex items-center justify-center mr-6 flex-shrink-0 font-bold text-lg">
    1
  </div>
  <div>
    <h3 class="text-xl font-bold text-slate-100 mb-3">
    <p class="text-slate-300 leading-relaxed">
```

### 7. Bullet Point with Icon
```
<li class="flex items-start">
  <svg class="w-5 h-5 text-emerald-400 mr-2 mt-0.5 flex-shrink-0">
  <span class="text-slate-300">
```

### 8. Icon Badge (for methodology cards)
```
<div class="w-16 h-16 bg-emerald-500/10 rounded-full flex items-center justify-center mx-auto mb-4 border border-emerald-500/20">
  <span class="text-emerald-400 font-semibold">1</span>
</div>
```

### 9. Policy Link Card
```
<a href="/privacy-policy"
   class="flex items-center px-4 py-3 border border-slate-600 rounded-md text-slate-200 hover:bg-slate-700 hover:border-emerald-500/50 transition-colors">
  <svg class="w-5 h-5 text-slate-400 mr-3">
  Text
</a>
```

---

## Recommended Theme Variable Structure

### Option 1: Tailwind Config Extension
Create custom color palette and component classes in `tailwind.config.js`

### Option 2: CSS Custom Properties
Create CSS variables in `global.css` that can be used with Tailwind's arbitrary values

### Option 3: Astro Component Props
Create reusable Astro components with consistent styling

### Recommended Approach: **CSS Custom Properties + Tailwind + Astro Components**
This approach gives us the best of all worlds and **full light/dark theme support**:

1. **CSS Custom Properties** - Define semantic color tokens that can change based on theme
2. **Tailwind Config** - Extend Tailwind to use our CSS variables
3. **Astro Components** - Reusable UI components that automatically adapt to theme

**Why this approach is theme-ready:**
- CSS variables can be scoped to `.dark` or `[data-theme="dark"]` selectors
- Adding light theme = just defining new CSS variable values
- Components don't need to change at all
- Can add theme toggle later without refactoring

---

## Light/Dark Theme Strategy

### Semantic Color Tokens (Theme-Agnostic Names)

Instead of `bg-slate-950`, we use semantic names like `bg-background-primary`:

```css
:root {
  /* Will be defined when light theme is added */
  --color-background-primary: /* light colors */;
  --color-text-primary: /* dark colors for contrast */;
}

[data-theme="dark"] {
  /* Current dark theme colors */
  --color-background-primary: rgb(2 6 23); /* slate-950 */
  --color-background-secondary: rgb(15 23 42); /* slate-900 */
  --color-background-tertiary: rgb(30 41 59); /* slate-800 */

  --color-text-primary: rgb(241 245 249); /* slate-100 */
  --color-text-secondary: rgb(203 213 225); /* slate-300 */
  --color-text-tertiary: rgb(148 163 184); /* slate-400 */
  --color-text-muted: rgb(100 116 139); /* slate-500 */

  --color-accent-emerald: rgb(52 211 153); /* emerald-400 */
  --color-accent-cyan: rgb(34 211 238); /* cyan-400 */

  --color-border-primary: rgb(51 65 85); /* slate-700 */
  --color-border-secondary: rgb(71 85 105); /* slate-600 */
}

/* When light theme is added (future): */
:root {
  --color-background-primary: rgb(255 255 255); /* white */
  --color-background-secondary: rgb(248 250 252); /* slate-50 */
  --color-background-tertiary: rgb(241 245 249); /* slate-100 */

  --color-text-primary: rgb(15 23 42); /* slate-900 */
  --color-text-secondary: rgb(51 65 85); /* slate-700 */
  --color-text-tertiary: rgb(100 116 139); /* slate-500 */
  --color-text-muted: rgb(148 163 184); /* slate-400 */

  --color-accent-emerald: rgb(16 185 129); /* emerald-500 */
  --color-accent-cyan: rgb(6 182 212); /* cyan-500 */

  --color-border-primary: rgb(226 232 240); /* slate-200 */
  --color-border-secondary: rgb(203 213 225); /* slate-300 */
}
```

### Tailwind Config Extension

```js
// tailwind.config.mjs
export default {
  theme: {
    extend: {
      colors: {
        'bg-primary': 'var(--color-background-primary)',
        'bg-secondary': 'var(--color-background-secondary)',
        'bg-tertiary': 'var(--color-background-tertiary)',

        'text-primary': 'var(--color-text-primary)',
        'text-secondary': 'var(--color-text-secondary)',
        'text-tertiary': 'var(--color-text-tertiary)',
        'text-muted': 'var(--color-text-muted)',

        'accent-emerald': 'var(--color-accent-emerald)',
        'accent-cyan': 'var(--color-accent-cyan)',

        'border-primary': 'var(--color-border-primary)',
        'border-secondary': 'var(--color-border-secondary)',
      }
    }
  }
}
```

### Usage in Components

```astro
<!-- Old way (hard-coded dark theme): -->
<div class="bg-slate-800 text-slate-100 border-slate-700">

<!-- New way (theme-agnostic): -->
<div class="bg-bg-tertiary text-text-primary border-border-primary">
```

### Benefits

1. **Current**: Works with dark theme immediately
2. **Future**: Add light theme by just defining `:root` CSS variables
3. **Flexible**: Can add theme toggle without touching components
4. **Maintainable**: Change one variable, update entire site
5. **Type-safe**: Tailwind autocomplete works with custom colors

---

## Migration Path

### Phase 1: Now (Dark Theme Only)
1. Define CSS variables under `[data-theme="dark"]`
2. Extend Tailwind config
3. Create Astro components using semantic color names
4. Refactor existing pages

### Phase 2: Future (Add Light Theme)
1. Define CSS variables under `:root` for light theme
2. Add theme toggle component
3. Set `data-theme` attribute on `<html>` tag
4. **No component changes needed!**

### Phase 3: Future (Theme Toggle)
1. Create theme switcher component
2. Store preference in localStorage
3. Add to header/footer
4. Optionally: respect `prefers-color-scheme`

---

## Proposed Component Library

1. **PageHeader.astro** - Page title and subtitle
2. **SectionHeader.astro** - Section heading with optional subtitle
3. **ContentBox.astro** - Main content container with prose styling
4. **Card.astro** - Basic card with variants (default, gradient, hover-accent)
5. **FeatureCard.astro** - Card with icon, title, description
6. **StepCard.astro** - Numbered step with title and description
7. **IconBadge.astro** - Circular icon container with number or icon
8. **BulletList.astro** - List with custom icon bullets
9. **PolicyLink.astro** - Link card for policy pages
10. **Section.astro** - Full section wrapper with background variants

---

## Next Steps

1. Decide on implementation approach
2. Create Tailwind config extensions for color palette
3. Build reusable Astro components
4. Refactor existing pages to use components
5. Document component usage
