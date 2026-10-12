# Provenance Graph — Storage & Pipeline Design

> **Context**: Desktop app (Rust + SQLite) that tracks the decision trail from PRD → Epics/Tasks → Code. Target users: business owners and PMs building software with coding agents.

---

## 1. Core Mental Model

Everything is a **Node** (artifact) connected by **Edges** (provenance relationships). Separate:

- **What** something is → node type + content
- **Where** it came from → edges
- **Why** it exists → edge type + rationale

The full PRD (or any large document) is **never stored as a single node**. It lives on disk as raw text. `nodes` stores semantic chunks of it. `document_tree` stores the hierarchy so you can reconstruct reading order and nesting.

---

## 2. SQLite Schema

```sql
-- ────────────────────────────────────────────────
-- 1. NODES  (every artifact)
-- ────────────────────────────────────────────────
CREATE TABLE nodes (
    id          TEXT PRIMARY KEY,          -- e.g. "prd:section-4.2", "chat:msg-uuid", "code:src/auth.py:L42-L67"
    type        TEXT NOT NULL,             -- prd_section | chat_message | decision | epic | task | code_span | question | answer | function_spec
    source      TEXT NOT NULL,             -- prd | slack | jira | github | claude_chat | email
    source_ref  TEXT,                      -- original ID in that system (Jira key, git SHA+path, etc.)

    title       TEXT,                      -- short human label
    body        TEXT,                      -- full text content
    body_hash   TEXT,                      -- sha256(body) for dedup / change detection

    author      TEXT,                      -- human name, "claude", "gpt-4", etc.
    author_type TEXT CHECK(author_type IN ('human','ai','system')),

    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL,

    -- for code nodes specifically
    file_path   TEXT,
    line_start  INTEGER,
    line_end    INTEGER,
    git_commit  TEXT,

    metadata    TEXT DEFAULT '{}'          -- JSON blob for anything extra
);

CREATE INDEX idx_nodes_type     ON nodes(type);
CREATE INDEX idx_nodes_source   ON nodes(source, source_ref);
CREATE INDEX idx_nodes_file     ON nodes(file_path, line_start);


-- ────────────────────────────────────────────────
-- 2. EDGES  (provenance relationships)
-- ────────────────────────────────────────────────
CREATE TABLE edges (
    id              TEXT PRIMARY KEY,
    from_node       TEXT NOT NULL REFERENCES nodes(id),
    to_node         TEXT NOT NULL REFERENCES nodes(id),

    rel             TEXT NOT NULL,
    -- Vocabulary:
    --   derived_from      | "this epic came from this PRD section"
    --   clarifies         | "this Q&A clarifies this PRD section"
    --   answered_by       | "this PRD section was clarified by this answer"
    --   implements        | "this code implements this task"
    --   references        | "this task references this PRD section"
    --   overrides         | "this decision supersedes that PRD section"
    --   generated_by      | "claude generated this code from this task"
    --   split_from        | "this sub-task was split from this epic"
    --   raised_in         | "this question was raised during this chat message"
    --   resolved_by       | "this open question was resolved by this decision node"
    --   same_as           | "this chunk is a duplicate of that chunk"

    rationale       TEXT,                  -- WHY this edge exists (human or AI authored)
    confidence      REAL DEFAULT 1.0,      -- 0-1, useful for AI-inferred edges
    inferred        INTEGER DEFAULT 0,     -- 0 = explicit, 1 = AI-inferred

    created_at      TEXT NOT NULL,
    created_by      TEXT,                  -- who/what created this edge

    metadata        TEXT DEFAULT '{}'
);

CREATE INDEX idx_edges_from     ON edges(from_node);
CREATE INDEX idx_edges_to       ON edges(to_node);
CREATE INDEX idx_edges_rel      ON edges(rel);
CREATE INDEX idx_edges_from_rel ON edges(from_node, rel);


-- ────────────────────────────────────────────────
-- 3. OPEN QUESTIONS  (first-class citizens)
-- ────────────────────────────────────────────────
CREATE TABLE question_lifecycle (
    question_node   TEXT PRIMARY KEY REFERENCES nodes(id),
    status          TEXT DEFAULT 'open'
        CHECK(status IN ('open','answered','wont_fix','moot')),
    answered_by     TEXT REFERENCES nodes(id),
    affects_nodes   TEXT DEFAULT '[]'      -- JSON array of node IDs whose design depends on this
);


-- ────────────────────────────────────────────────
-- 4. DOCUMENT STRUCTURE  (hierarchy for large docs)
-- ────────────────────────────────────────────────
CREATE TABLE document_tree (
    node_id     TEXT PRIMARY KEY REFERENCES nodes(id),
    parent_id   TEXT REFERENCES nodes(id),
    doc_root    TEXT NOT NULL REFERENCES nodes(id),
    seq_order   REAL NOT NULL,             -- fractional for easy reordering
    depth       INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_doctree_parent ON document_tree(parent_id);
CREATE INDEX idx_doctree_root   ON document_tree(doc_root, seq_order);


-- ────────────────────────────────────────────────
-- 5. FULL-TEXT SEARCH
-- ────────────────────────────────────────────────
CREATE VIRTUAL TABLE nodes_fts USING fts5(
    id UNINDEXED,
    title,
    body,
    author,
    content='nodes',
    content_rowid='rowid',
    tokenize='porter unicode61'
);

CREATE TRIGGER nodes_fts_insert AFTER INSERT ON nodes BEGIN
    INSERT INTO nodes_fts(rowid, id, title, body, author)
    VALUES (new.rowid, new.id, new.title, new.body, new.author);
END;
CREATE TRIGGER nodes_fts_update AFTER UPDATE ON nodes BEGIN
    UPDATE nodes_fts SET title=new.title, body=new.body, author=new.author
    WHERE id=new.id;
END;


-- ────────────────────────────────────────────────
-- 6. TAGS
-- ────────────────────────────────────────────────
CREATE TABLE tags (
    node_id     TEXT NOT NULL REFERENCES nodes(id),
    tag         TEXT NOT NULL,
    PRIMARY KEY (node_id, tag)
);
CREATE INDEX idx_tags_tag ON tags(tag);


-- ────────────────────────────────────────────────
-- 7. LLM CALL CACHE  (reruns cost nothing)
-- ────────────────────────────────────────────────
CREATE TABLE llm_cache (
    prompt_hash TEXT PRIMARY KEY,          -- sha256(model + prompt)
    model       TEXT NOT NULL,
    prompt      TEXT NOT NULL,
    response    TEXT NOT NULL,
    created_at  TEXT NOT NULL
);
```

---

## 3. Node ID Convention

Use readable, stable IDs so edges are human-inspectable without querying.

| Type | ID format |
|---|---|
| PRD section | `prd:{doc-slug}:s{section-num}` |
| Chat message | `chat:{session-id}:{seq}` |
| Decision | `decision:{slug}:{yyyymmdd}` |
| Jira Epic | `jira:epic:{key}` |
| Jira Task | `jira:task:{key}` |
| Code span | `code:{relative/path}:L{start}-{end}@{short-sha}` |
| Question | `question:{slug}` |
| Function spec | `fnspec:{function-name}` |

---

## 4. Chunking Strategy

### The rule
A node should be **the smallest thing you'd want to cite**. The test: "Would I ever draw an edge to just this chunk, and not its neighbours?" If yes, it deserves its own node.

### Chunk boundaries by source

| Source | Boundary |
|---|---|
| PRD | Named rule, constraint, or decision — not headings, not paragraphs |
| Slack / chat | Topic shift, not individual messages. One Q&A exchange = one or two nodes |
| Jira ticket | Description = one node. Each decision-bearing comment = separate node |
| Claude chat session | Each distinct Q&A pair = one node. Session = parent in `document_tree` |
| Code | Function body. For large functions, logical block if it maps to a distinct rule |

### Chunk sizes to avoid

- **Too large**: "Section 4 — Unit List Approval" as a single node loses precision on edges
- **Too small**: Single sentences with no standalone meaning ("See Section 6.6")

### Example — one PRD passage → five nodes

Original text:
> Before a Property becomes active for rent roll uploads, OSC reviews the Property Manager's submitted unit list and either (a) approves, (b) rejects with feedback, or (c) requests revision. The approved unit list locks the unit set for that Property. After initial OSC approval, the unit list is locked for the duration of MVP. Any subsequent unit additions, modifications, or deletions are handled out-of-band by OSC and logged in the audit log. A PM-facing change request workflow is Phase 1. The PM cannot upload a rent roll until the unit list is approved.

Chunks:

```
prd:mvp:s4.2.1  "OSC review actions: approve / reject / request revision"
prd:mvp:s4.2.2  "Approved unit list locks the unit set for the Property"
prd:mvp:s4.2.3  "Post-approval changes are out-of-band OSC edits, logged in audit log"
prd:mvp:s4.2.4  "PM cannot upload rent roll until unit list is approved"
prd:mvp:s4.2.5  "PM-facing change request workflow is deferred to Phase 1"
```

---

## 5. From Chunks to function_specs

### The rule
One function = one decision the system makes about one entity at one point in its lifecycle.

Rules that are **alternatives/branches of the same decision** → same function (internal branching).
Rules that govern the **same entity at the same lifecycle stage** → same function or same module.
Rules that **supersede older rules** → same function updated; old version lives in the graph via an `overrides` edge, not in the code.

### Example — five PRD chunks → function specs

```
fnspec:apply_osc_decision
  sources: prd:mvp:s4.2.1, prd:mvp:s4.2.2
  signature: apply_osc_decision(unit_list, action, feedback) -> UnitList
  description: Takes a submitted unit list and an OSC action.
               Approve → locks unit set.
               Reject → resets to pending, PM must resubmit from scratch.
               Request revision → stays editable, PM amends in place.

fnspec:can_pm_upload_rent_roll
  sources: prd:mvp:s4.2.4
  signature: can_pm_upload_rent_roll(unit_list) -> bool

fnspec:is_unit_list_locked
  sources: prd:mvp:s4.2.2, prd:mvp:s4.2.3
  signature: is_unit_list_locked(unit_list) -> bool
```

Store these as `function_spec` nodes before any code is written. When code is generated, add a `code` node that `implements` the spec. The PRD → spec → code chain is fully traceable.

---

## 6. Multi-Pass Ingestion Pipeline

```
Raw Text
   │
   ▼
Pass 1: Structure Extraction          [deterministic]
   │
   ▼
Pass 2: Sentence Segmentation         [deterministic]
   │
   ▼
Pass 3: Semantic Boundary Detection   [tiny ML model]
   │
   ▼
Pass 4: Chunk Classification          [rules + tiny LLM fallback]
   │
   ▼
Pass 5: Entity + Verb Extraction      [deterministic NLP]
   │
   ▼
Pass 6: Relation Inference            [tiny LLM — most important]
   │
   ▼
Pass 7: function_spec Synthesis       [tiny LLM]
   │
   ▼
Pass 8: Embedding + Dedup             [embedding model]
   │
   ▼
Nodes + Edges in SQLite
```

### Pass 1 — Structure Extraction `[deterministic, Rust]`

Parse document format to extract hierarchy before touching content.

- Markdown → heading levels give the tree directly
- Jira export → ticket → subtask is the tree
- Chat JSON → session → message → reply-thread

Output: rows in `document_tree`. No ML needed.

### Pass 2 — Sentence Segmentation `[deterministic, Rust]`

Split paragraph blobs into sentences using rule-based boundary detection. Use the `unicode-segmentation` crate plus a small set of abbreviation exceptions (`e.g.`, `vs.`, `Sec.`). Handles 95% of PRD text correctly.

Output: flat temp table of `(sentence_id, paragraph_id, text, position)`. Not yet nodes — candidates only.

### Pass 3 — Semantic Boundary Detection `[tiny ML]`

Decides which sentences belong together as one chunk. Model answers: *does the next sentence continue the same thought, or start a new one?*

Options (in order of preference for structured docs):

1. **TextTiling** — fully deterministic, uses term-repetition to find topic shifts. No model. Surprisingly good for PRDs.
2. **Bi-encoder classifier** on top of `all-MiniLM-L6-v2` (22MB). Input: sentence pair embeddings. Output: `continue | break`.

For PRDs, TextTiling + heading boundaries from Pass 1 is sufficient. Reserve the ML model for unstructured sources like Slack threads.

Output: chunk boundaries → grouped sentence clusters.

### Pass 4 — Chunk Classification `[rules first, tiny LLM fallback]`

Classify each chunk into a node type: `rule | decision | question | definition | reference | rationale | deferred`.

**Rule-based patterns (Rust, zero model cost):**

```
"shall" | "must" | "cannot" | "is required"   →  rule
"TBD" | "open question" | "to be decided"     →  question
"See Section" | "per Section"                 →  reference
"Phase 1" | "future" | "out of scope"         →  deferred
"means" | "is defined as" | "refers to"       →  definition
```

Rules classify ~70% of PRD chunks. Only unmatched chunks go to the LLM.

**LLM fallback**: Phi-3-mini (3.8B, Q4 quantized, ~2GB) via `llama.cpp`. Single classification prompt, one-token output.

### Pass 5 — Entity + Verb Extraction `[deterministic NLP]`

For each `rule` chunk, extract:

- **Subject entity** — what thing does this rule govern? (`unit_list`, `property`, `PM`, `OSC`)
- **Action verb** — what does it do or constrain? (`locks`, `cannot upload`, `is approved`)
- **Lifecycle stage** — what state is the entity in? (`submitted`, `approved`, `pending`)

Use `rust-bert` with a pre-trained NER model, or a noun-phrase chunker plus a curated entity dictionary that grows as you ingest documents.

Output: tags on each chunk node — `entity:unit_list`, `verb:lock`, `stage:post_approval`. These tags drive Pass 6 and 7.

### Pass 6 — Relation Inference `[tiny LLM — most important pass]`

Infer edges between chunks:

- Does chunk A *override* chunk B (same rule, newer version)?
- Does chunk A *clarify* chunk B?
- Does chunk A *depend on* chunk B?

**Run only on pairs that share an entity tag from Pass 5.** This keeps pairs at O(chunks_per_entity²) not O(all_chunks²).

Prompt (Qwen2.5-1.5B or Phi-3-mini):

```
Chunk A: "Approved unit list locks the unit set for that Property."
Chunk B: "Any subsequent unit additions are handled out-of-band by OSC."
Relationship options: [overrides, clarifies, depends_on, implements, unrelated]
Answer with one word.
```

Store inferred edges with `confidence < 1.0` and `inferred = 1`. Human reviewers can spot-check low-confidence edges.

### Pass 7 — function_spec Synthesis `[tiny LLM]`

Group chunks by `(entity, lifecycle_stage)` from Pass 5 tags. Each group is a candidate function.

Prompt:

```
These rules all govern the same entity and lifecycle stage:
1. "OSC can approve, reject, or request revision."
2. "Approved unit list locks the unit set."
3. "PM cannot upload rent roll until approved."

Write a function signature and one-paragraph description for a pure function
that implements all of these rules. Output JSON only:
{"name": "...", "signature": "...", "description": "..."}
```

Output goes into a `function_spec` node. Edges from each source chunk to the spec are drawn automatically from the grouping.

### Pass 8 — Embedding + Dedup `[embedding model]`

Two jobs:

**Dedup**: when ingesting a new document, compute cosine similarity against existing chunk embeddings. Threshold ~0.92. If duplicate, draw a `same_as` edge instead of creating a new node.

**Search index**: store embeddings using the `sqlite-vec` SQLite extension. Enables vector search without a separate database.

Model: `all-MiniLM-L6-v2` (22MB, 384 dimensions). Use the `fastembed-rs` crate — wraps ONNX runtime, runs natively in Rust, no Python dependency.

---

## 7. Performance

| Pass | Model | Time per chunk | Notes |
|---|---|---|---|
| 1–2 | None | <1ms | I/O bound |
| 3 | TextTiling / MiniLM | ~2ms | CPU |
| 4 | Rules + Phi-3 fallback | <1ms / ~200ms | LLM only for unmatched chunks |
| 5 | NER | ~5ms | CPU |
| 6 | Qwen2.5-1.5B | ~150ms per pair | **Batch this** |
| 7 | Qwen2.5-1.5B | ~300ms per group | **Batch this** |
| 8 | MiniLM | ~3ms | CPU |

**Optimisations:**

- Cache all LLM outputs in `llm_cache` keyed on `sha256(model + prompt)`. Reruns on unchanged documents cost nothing.
- Run Pass 6/7 async while passes 1–5 continue on the next document.
- Pass 6 entity-matching filter eliminates the majority of pairs before any LLM call.

A 50-page PRD (~300 chunks) completes the full pipeline in under 2 minutes on a modern laptop CPU with a quantized model.

---

## 8. Rust Crates

```toml
[dependencies]
unicode-segmentation = "1"     # Pass 2 — sentence boundary detection
fastembed = "3"                # Pass 3, 8 — ONNX embedding, no Python required
rust-bert = "0.21"             # Pass 5 — NER (or use candle for lighter weight)
sqlite-vec = "0.1"             # Pass 8 — vector search inside SQLite
rusqlite = "0.31"              # SQLite driver
tokio = { version = "1", features = ["full"] }  # async pipeline orchestration
serde_json = "1"               # LLM prompt/response parsing
sha2 = "0.10"                  # prompt hashing for llm_cache
```

For LLM passes, run `llama.cpp` or `ollama` as a local HTTP server and call it from Rust. This keeps the model runtime separate from the core engine and lets you swap models without recompiling.

---

## 9. Key Queries (no LLM needed at query time)

```sql
-- "Why does this line of code exist?" — walk ancestry
WITH RECURSIVE ancestry(id, rel, depth) AS (
    SELECT from_node, rel, 0 FROM edges WHERE to_node = 'code:models/unit_list.py:L12-34'
    UNION ALL
    SELECT e.from_node, e.rel, a.depth+1
    FROM edges e JOIN ancestry a ON e.to_node = a.id
    WHERE a.depth < 10
)
SELECT n.type, n.title, n.body, a.rel, a.depth
FROM ancestry a JOIN nodes n ON n.id = a.id
ORDER BY a.depth;

-- "All open questions that affect this epic"
SELECT n.* FROM nodes n
JOIN edges e ON e.from_node = n.id
JOIN question_lifecycle ql ON ql.question_node = n.id
WHERE e.to_node = 'epic:unit-list-approval'
  AND n.type = 'question'
  AND ql.status = 'open';

-- "Which function specs are sourced from this PRD section?"
SELECT n.* FROM nodes n
JOIN edges e ON e.to_node = n.id
WHERE e.from_node = 'prd:mvp:s4.2.1'
  AND n.type = 'function_spec';

-- "Everything Claude generated vs human decided"
SELECT type, count(*) FROM nodes
WHERE author_type = 'ai'
GROUP BY type;

-- Full-text search across all artifacts
SELECT n.id, n.type, n.title,
       snippet(nodes_fts, 2, '<b>','</b>','…', 20) AS excerpt
FROM nodes_fts JOIN nodes n ON n.id = nodes_fts.id
WHERE nodes_fts MATCH 'unit list approval'
ORDER BY rank;
```

---

## 10. What Not to Store in SQLite

| Artefact | Storage |
|---|---|
| Raw binary files (PDFs, images) | Disk; store path in `source_ref` |
| Git history | Query via git directly; store only commit SHA + span in nodes |
| Vector embeddings | `sqlite-vec` extension inside the same SQLite file |
| Full document text | Disk or a separate `documents` table; nodes hold chunks only |
