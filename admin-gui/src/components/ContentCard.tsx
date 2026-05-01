import { JSX, Show } from 'solid-js';

export interface ContentCardProps {
  title: string;
  body?: string | null;
  meta?: string | null;
  /** Renders an icon or avatar to the left of the text */
  leading?: JSX.Element;
  selected?: boolean;
  onClick?: () => void;
  disabled?: boolean;
}

export function ContentCard(props: ContentCardProps) {
  return (
    <div
      class="content-card"
      classList={{
        'content-card--selected': !!props.selected,
        'content-card--clickable': !!props.onClick,
        'content-card--disabled': !!props.disabled,
      }}
      onClick={() => !props.disabled && props.onClick?.()}
    >
      <Show when={props.leading}>
        <div class="content-card-leading">{props.leading}</div>
      </Show>
      <div class="content-card-body">
        <div class="content-card-title">{props.title}</div>
        <Show when={props.body}>
          <div class="content-card-text">{props.body}</div>
        </Show>
      </div>
      <Show when={props.meta}>
        <div class="content-card-meta">{props.meta}</div>
      </Show>
    </div>
  );
}
