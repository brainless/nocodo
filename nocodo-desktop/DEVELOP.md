# Development Notes

## Slint UI Layout Patterns

### Full-Width Page Backgrounds with Fixed-Width Content

When creating pages with fixed-width widgets, wrap them in `HorizontalLayout` to prevent parent layout shrinking:

```slint
export component MyPage inherits Rectangle {
    background: DesktopColors.background;
    horizontal-stretch: 1;
    min-width: 0;  // Prevents content-based shrinking

    VerticalLayout {
        // Wrap fixed-width content in HorizontalLayout
        HorizontalLayout {
            alignment: start;  // or center

            Rectangle {
                width: 300px;  // Fixed width
                // ... content
            }
        }
    }
}
```

**Why:** Placing fixed-width elements directly in `VerticalLayout` causes the parent `Rectangle` to shrink to content width. Wrapping in `HorizontalLayout` allows the layout to fill available width while positioning the fixed-width content according to alignment.

**Reference:** See `ChatsPage` and `SettingsPage` for working examples.
