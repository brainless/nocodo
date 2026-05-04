import { For, Show, createEffect, createSignal } from 'solid-js';
import { useParams } from '@solidjs/router';
import { useProject } from '../contexts/ProjectContext';

const API_BASE_URL = '';

// ---- Types ------------------------------------------------------------------

type FormFieldType = 'text' | 'number' | 'boolean' | 'date' | 'select' | 'textarea';

interface FormField {
  name: string;
  label: string;
  field_type: FormFieldType;
  required: boolean;
  placeholder?: string;
}

interface FormRow {
  fields: FormField[];
}

interface FormLayout {
  entity: string;
  title: string;
  rows: FormRow[];
}

interface FormLayoutResponse {
  entity_name: string;
  layout: FormLayout;
}

// ---- Form Canvas ------------------------------------------------------------

function FieldSkeleton(props: { field: FormField }) {
  return (
    <div class="uid-field">
      <div class="uid-label">{props.field.label}{props.field.required && <span class="uid-required">*</span>}</div>
      {props.field.field_type === 'boolean' ? (
        <div class="uid-checkbox-row">
          <div class="uid-checkbox" />
          <div class="uid-checkbox-label">{props.field.label}</div>
        </div>
      ) : props.field.field_type === 'textarea' ? (
        <div class="uid-textarea" />
      ) : props.field.field_type === 'select' ? (
        <div class="uid-select">
          <div class="uid-select-chevron">▾</div>
        </div>
      ) : (
        <div class="uid-input" />
      )}
    </div>
  );
}

function FormCanvas(props: { layout: FormLayout }) {
  return (
    <div class="uid-canvas">
      <div class="uid-form-title">{props.layout.title}</div>
      <For each={props.layout.rows}>
        {row => (
          <div class="uid-row">
            <For each={row.fields}>
              {field => <FieldSkeleton field={field} />}
            </For>
          </div>
        )}
      </For>
      <div class="uid-actions">
        <div class="uid-btn-primary">Save</div>
        <div class="uid-btn-ghost">Cancel</div>
      </div>
    </div>
  );
}

// ---- Entity Card ------------------------------------------------------------

function EntityCard(props: { entityName: string; projectId: number }) {
  const [layout, setLayout] = createSignal<FormLayout | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const fetchLayout = async () => {
    try {
      const res = await fetch(
        `${API_BASE_URL}/api/agents/ui-designer/form/${props.projectId}/${props.entityName}`
      );
      if (res.ok) {
        const data = await res.json() as FormLayoutResponse;
        setLayout(data.layout);
        return true;
      }
    } catch {}
    return false;
  };

  const generateForm = async () => {
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(`${API_BASE_URL}/api/agents/ui-designer/form`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ project_id: props.projectId, entity_name: props.entityName }),
      });

      if (res.ok) {
        const data = await res.json();
        // 200 = cached form returned immediately
        if (data.layout) {
          setLayout(data.layout);
          setLoading(false);
          return;
        }
        // 202 = accepted, poll until ready
        const poll = async () => {
          const found = await fetchLayout();
          if (found) {
            setLoading(false);
          } else {
            setTimeout(poll, 1500);
          }
        };
        poll();
      } else {
        const err = await res.json().catch(() => ({ error: 'Unknown error' }));
        setError(err.error ?? 'Generation failed');
        setLoading(false);
      }
    } catch (e) {
      setError(String(e));
      setLoading(false);
    }
  };

  // Check cache on mount.
  createEffect(() => {
    fetchLayout();
  });

  return (
    <div class="uid-entity-card">
      <div class="uid-entity-header">
        <div class="uid-entity-name">{props.entityName}</div>
        <Show when={!layout()}>
          <button class="uid-gen-btn" onClick={generateForm} disabled={loading()}>
            {loading() ? 'Generating…' : 'Generate Form'}
          </button>
        </Show>
        <Show when={layout()}>
          <button class="uid-regen-btn" onClick={() => { setLayout(null); generateForm(); }} disabled={loading()}>
            ↺
          </button>
        </Show>
      </div>
      <Show when={error()}>
        <div class="uid-error">{error()}</div>
      </Show>
      <Show when={loading() && !layout()}>
        <div class="uid-generating">
          <div class="uid-spinner" />
          <span>Designing form…</span>
        </div>
      </Show>
      <Show when={layout()}>
        <FormCanvas layout={layout()!} />
      </Show>
    </div>
  );
}

// ---- Page -------------------------------------------------------------------

export default function UIDesignerPage() {
  const params = useParams();
  const { currentProject } = useProject();
  const [entities, setEntities] = createSignal<string[]>([]);
  const [loadError, setLoadError] = createSignal<string | null>(null);

  const projectId = () => {
    const id = params.projectId ? Number(params.projectId) : currentProject()?.id;
    return id ?? null;
  };

  createEffect(() => {
    const pid = projectId();
    if (!pid) return;

    fetch(`${API_BASE_URL}/api/agents/ui-designer/entities/${pid}`)
      .then(r => r.json())
      .then((data: { entities: string[] }) => setEntities(data.entities ?? []))
      .catch(e => setLoadError(String(e)));
  });

  return (
    <div class="uid-page">
      <div class="uid-page-header">
        <h2 class="uid-page-title">UI Designer</h2>
        <p class="uid-page-subtitle">Generate low-fidelity form layouts for your entities.</p>
      </div>
      <Show when={loadError()}>
        <div class="uid-error uid-error-block">{loadError()}</div>
      </Show>
      <Show when={!entities().length && !loadError()}>
        <div class="uid-empty">
          No schema found. Run the DB Developer first to design your data model.
        </div>
      </Show>
      <div class="uid-entity-grid">
        <For each={entities()}>
          {name => <EntityCard entityName={name} projectId={projectId()!} />}
        </For>
      </div>
    </div>
  );
}
