# nocodo Theme Components Guide

## Overview

This design system provides reusable, theme-aware components that automatically adapt to light/dark modes. All components use semantic color tokens, making it easy to add light theme support in the future without changing any component code.

## Quick Start

```astro
---
import PageHeader from '../components/theme/PageHeader.astro';
import ContentBox from '../components/theme/ContentBox.astro';
import Card from '../components/theme/Card.astro';
---

<PageHeader
  title="My Page"
  subtitle="Page description"
/>

<ContentBox>
  <p>Content goes here</p>
</ContentBox>
```

## Available Components

### 1. Button
Theme-aware button component with multiple variants and sizes.

**Props:**
- `variant` ('primary' | 'secondary' | 'accent-emerald' | 'accent-cyan' | 'outline' | 'ghost', default: 'primary')
- `size` ('sm' | 'md' | 'lg', default: 'md')
- `disabled` (boolean, default: false)
- `href` (string, optional): For link-style buttons
- `type` ('button' | 'submit' | 'reset', default: 'button')
- `class` (string, optional): Additional CSS classes

**Example:**
```astro
<Button
  href="/events"
  variant="accent-emerald"
  size="sm"
>
  Join Event
</Button>
```

---

### 2. NavLink
Theme-aware navigation link with active state detection.

**Props:**
- `href` (string, required): Link destination
- `active` (boolean, optional): Manually set active state
- `class` (string, optional): Additional CSS classes

**Example:**
```astro
<NavLink href="/about">
  About
</NavLink>
```

---

### 3. PageHeader
Main page title and subtitle.

**Props:**
- `title` (string, required): Page title
- `subtitle` (string, optional): Page subtitle
- `centered` (boolean, default: true): Center alignment

**Example:**
```astro
<PageHeader
  title="About Us"
  subtitle="Learn about our mission and values"
/>
```

---

### 4. SectionHeader
Section heading with optional subtitle.

**Props:**
- `title` (string, required): Section title
- `subtitle` (string, optional): Section subtitle
- `centered` (boolean, default: true): Center alignment

**Example:**
```astro
<SectionHeader
  title="Core Features"
  subtitle="Everything you need to succeed"
/>
```

---

### 5. ContentBox
Container for main content with prose styling. Ideal for markdown/rich text.

**Props:**
- `class` (string, optional): Additional CSS classes

**Example:**
```astro
<ContentBox>
  <h2>Privacy Policy</h2>
  <p>Your privacy is important to us...</p>
</ContentBox>
```

---

### 6. Card
Reusable card component with three variants.

**Props:**
- `variant` ('default' | 'gradient' | 'hover-accent', default: 'default')
- `accentColor` ('emerald' | 'cyan', default: 'emerald')
- `class` (string, optional): Additional CSS classes

**Variants:**
- `default`: Simple card with border
- `gradient`: Card with gradient background
- `hover-accent`: Border changes to accent color on hover

**Example:**
```astro
<Card variant="hover-accent" accentColor="cyan">
  <h3>Feature Title</h3>
  <p>Feature description</p>
</Card>
```

---

### 7. FeatureCard
Card with icon, title, and description.

**Props:**
- `title` (string, required): Feature title
- `description` (string, required): Feature description
- `iconColor` ('emerald' | 'cyan', default: 'emerald')
- `variant` ('default' | 'gradient', default: 'default')
- `class` (string, optional): Additional CSS classes

**Slots:**
- `icon`: Custom SVG icon (optional, has default)

**Example:**
```astro
<FeatureCard
  title="Fast Performance"
  description="Lightning-fast load times"
  iconColor="emerald"
>
  <svg slot="icon" class="w-6 h-6 text-accent-emerald" fill="none" stroke="currentColor" viewBox="0 0 24 24">
    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"></path>
  </svg>
</FeatureCard>
```

---

### 8. IconBadge
Icon container with optional number badge.

**Props:**
- `size` ('sm' | 'md' | 'lg', default: 'md')
- `shape` ('square' | 'circle', default: 'square')
- `color` ('emerald' | 'cyan', default: 'emerald')
- `number` (string | number, optional): Number to display
- `class` (string, optional): Additional CSS classes

**Example:**
```astro
<!-- With number -->
<IconBadge number="1" shape="circle" />

<!-- With custom icon -->
<IconBadge color="cyan">
  <svg class="w-6 h-6 text-accent-cyan">...</svg>
</IconBadge>
```

---

### 9. Section
Full-width section wrapper with background variants.

**Props:**
- `background` ('primary' | 'secondary' | 'gradient', default: 'primary')
- `containerWidth` ('sm' | 'md' | 'lg' | 'xl' | '2xl' | '4xl' | '5xl' | '6xl', default: '6xl')
- `paddingY` ('sm' | 'md' | 'lg', default: 'lg')
- `class` (string, optional): Additional CSS classes

**Example:**
```astro
<Section background="secondary" containerWidth="4xl">
  <SectionHeader title="Our Services" />
  <!-- Section content -->
</Section>
```

---

## Color Tokens

All components use semantic color tokens that are theme-aware:

### Background Colors
- `bg-bg-primary` - Primary background (slate-950)
- `bg-bg-secondary` - Secondary background (slate-900)
- `bg-bg-tertiary` - Tertiary background (slate-800)

### Text Colors
- `text-text-primary` - Primary text (slate-100)
- `text-text-secondary` - Secondary text (slate-300)
- `text-text-tertiary` - Tertiary text (slate-400)
- `text-text-muted` - Muted text (slate-500)

### Accent Colors
- `text-accent-emerald` - Emerald accent (emerald-400)
- `text-accent-cyan` - Cyan accent (cyan-400)
- `bg-accent-emerald` - Emerald background
- `bg-accent-cyan` - Cyan background

### Border Colors
- `border-border-primary` - Primary border (slate-700)
- `border-border-secondary` - Secondary border (slate-600)

### Surface Colors
- `bg-surface` - Surface background (slate-800)
- `bg-surface-hover` - Surface hover state (slate-700)

---

## Adding Light Theme (Future)

When you're ready to add light theme support:

1. **Define light theme CSS variables** in `global.css`:
```css
:root {
  --color-background-primary: 255 255 255; /* white */
  --color-text-primary: 15 23 42; /* slate-900 */
  /* ... etc */
}
```

2. **Add theme toggle component** to switch between themes

3. **Set data-theme attribute** on `<html>` tag:
```html
<html data-theme="dark">  <!-- or remove for light -->
```

4. **No component changes needed!** All components automatically adapt.

---

## Migration Guide

### Before (hard-coded colors):
```astro
<div class="bg-slate-800 text-slate-100 border-slate-700">
  <h2 class="text-slate-100">Title</h2>
  <p class="text-slate-300">Description</p>
</div>
```

### After (theme-aware):
```astro
<Card>
  <h2 class="text-text-primary">Title</h2>
  <p class="text-text-secondary">Description</p>
</Card>
```

Or using ContentBox:
```astro
<ContentBox>
  <h2>Title</h2>
  <p>Description</p>
</ContentBox>
```

---

## Best Practices

1. **Use components when possible** - Components ensure consistency and reduce code duplication

2. **Use semantic color tokens** - Instead of `bg-slate-800`, use `bg-bg-tertiary`

3. **Let components handle styling** - Avoid overriding component styles unless necessary

4. **Extend with classes** - Use the `class` prop to add additional styling when needed

5. **Keep content accessible** - Components maintain proper heading hierarchy and ARIA attributes

---

## Examples

### Simple Page Layout
```astro
---
import Layout from '../layouts/Layout.astro';
import PageHeader from '../components/theme/PageHeader.astro';
import Section from '../components/theme/Section.astro';
import SectionHeader from '../components/theme/SectionHeader.astro';
import Card from '../components/theme/Card.astro';
---

<Layout>
  <Section background="gradient">
    <PageHeader
      title="Welcome"
      subtitle="Get started with our platform"
    />
  </Section>

  <Section background="secondary">
    <SectionHeader title="Features" />
    <div class="grid md:grid-cols-3 gap-6">
      <Card variant="hover-accent">
        <h3 class="text-text-primary">Feature 1</h3>
        <p class="text-text-secondary">Description</p>
      </Card>
      <!-- More cards... -->
    </div>
  </Section>
</Layout>
```

### Feature Grid with Icons
```astro
<div class="grid md:grid-cols-2 gap-8">
  <FeatureCard
    title="Fast Performance"
    description="Lightning-fast load times"
    iconColor="emerald"
  />
  <FeatureCard
    title="Secure by Default"
    description="Built with security in mind"
    iconColor="cyan"
  />
</div>
```

### Numbered Steps
```astro
<div class="space-y-6">
  <div class="flex items-start gap-4">
    <IconBadge number="1" shape="circle" />
    <div>
      <h3 class="text-text-primary">Step One</h3>
      <p class="text-text-secondary">Do this first</p>
    </div>
  </div>
  <!-- More steps... -->
</div>
```
