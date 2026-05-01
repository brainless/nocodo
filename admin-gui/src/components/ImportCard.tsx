import { JSX, Show } from 'solid-js';

export type ImportCardTheme = 'blue' | 'green' | 'orange';

export interface ImportCardProps {
  icon: JSX.Element;
  theme: ImportCardTheme;
  title: string;
  description: JSX.Element | string;
  badge?: string;
  onClick?: () => void;
}

export function ImportCard(props: ImportCardProps) {
  return (
    <div
      class="import-card"
      classList={{ 'import-card--clickable': !!props.onClick }}
      onClick={props.onClick}
    >
      <div class={`import-card-icon import-card-icon--${props.theme}`}>
        {props.icon}
      </div>
      <div class="import-card-text">
        <h3>{props.title}</h3>
        <p>{props.description}</p>
      </div>
      <Show when={props.badge}>
        <div class="import-card-badge">{props.badge}</div>
      </Show>
    </div>
  );
}
