use crate::code_extractor::CodeBlock;

/// Extract the table name from a Diesel struct's `#[diesel(table_name = X)]` attribute.
pub fn extract_table_name(struct_code: &str) -> Option<String> {
    let start = struct_code.find("table_name = ")?;
    let rest = &struct_code[start + "table_name = ".len()..];
    let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')?;
    Some(rest[..end].to_string())
}

/// Extract the struct's field names as the canonical column list.
/// These are the ONLY valid column names for `table_name::column` references.
pub fn extract_column_names(struct_code: &str) -> Vec<String> {
    let mut columns = Vec::new();
    for line in struct_code.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("pub ") && trimmed.contains(':') {
            if let Some(name) = trimmed
                .strip_prefix("pub ")
                .and_then(|s| s.split(':').next())
            {
                let name = name.trim();
                if !name.is_empty() && !name.starts_with('_') {
                    columns.push(name.to_string());
                }
            }
        }
    }
    columns
}

/// Build a single-shot prompt for generating a Diesel model impl function.
///
/// Optimized for Qwen 3.5 0.8B: explicit rules, concrete Diesel+SQLite examples,
/// no ambiguity. Every pattern the model might need is shown.
///
/// Returns `(prompt, table_name)` — the caller prepends imports deterministically.
pub fn build_prompt(
    struct_code: &str,
    example_fns: &[CodeBlock],
    dependent_types: &[CodeBlock],
    fn_name: &str,
) -> (String, Option<String>) {
    let table_name = extract_table_name(struct_code);
    let columns = extract_column_names(struct_code);
    let mut prompt = String::new();

    // ── Role and task ──────────────────────────────────────────────────────
    prompt.push_str(
        r#"You are a Rust expert writing Diesel ORM functions for SQLite.

Write ONLY the function definition. Do NOT include imports — they are added automatically.
Return ONLY the function. No explanation, no markdown fences.

## Diesel + SQLite Rules

1. Connection type is `&mut SqliteConnection`
2. Use `diesel::insert_into`, `diesel::update`, `diesel::delete` for mutations
3. Use `.filter()`, `.find()`, `.limit()`, `.order()` for queries
4. Always use `.select(Model::as_select())` when loading structs
5. Always use `.returning(Model::as_returning())` when inserting/updating and returning the struct
6. Use `.get_result(conn)` for single row, `.load(conn)` for multiple rows
7. Use `.optional()` after `.first(conn)` to get `Option<Model>` instead of an error on not found
8. SQLite uses `i32` for INTEGER columns, not `i64`
9. SQLite uses `String` for TEXT columns, `bool` for BOOLEAN, `Option<T>` for NULLABLE

## Column Reference Rules (CRITICAL)

10. ALWAYS use `table_name::column_name` for column references (e.g. `users::id`, `users::email`)
11. NEVER invent column names — only use columns listed in "Available columns" below
12. NEVER use enum variant names as columns (e.g. `ContactType::Phone` does NOT mean a `phone` column exists)
13. When filtering on enum columns, use `EnumType::Variant.as_str()` against the enum column (e.g. `contacts::contact_type.eq(ContactType::Email.as_str())`)
14. Parameter names and column names are DIFFERENT — always use `table::column.eq(param_name)`
15. ONLY filter on columns for which you have a function parameter. Do NOT add filters for columns without parameters.
16. NEVER call methods on `pool` other than `pool.get()`. `pool.get()` returns a connection — it has no `.id()` method.

## Function Signature Pattern

Functions take `pool: &DbPool` and parameters. Get the connection FIRST, then use `conn` for all Diesel calls:

```rust
pub fn find_by_x(pool: &DbPool, x_value: &str) -> Result<Option<Self>, diesel::result::Error> {
    let mut conn = pool.get().expect("Failed to get connection");
    table_name
        .filter(table_name::column.eq(x_value))
        .select(Self::as_select())
        .first::<Self>(&mut conn)
        .optional()
}
```

NEVER write `pool.get().id()`, `pool.id()`, or any method on `pool` other than `pool.get()`.

"#,
    );

    // ── Available columns for the target table ─────────────────────────────
    if !columns.is_empty() {
        prompt.push_str(&format!(
            "## Available Columns for `{}`\n\n{}\n\n",
            table_name.as_deref().unwrap_or("unknown"),
            columns
                .iter()
                .map(|c| format!("- `{c}`"))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }

    // ── Struct/derive examples ─────────────────────────────────────────────
    prompt.push_str(
        r#"## Diesel Model Struct Examples

Basic Queryable + Insertable:
```rust
#[derive(Queryable, Selectable)]
#[diesel(table_name = posts)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub body: String,
    pub published: bool,
}

#[derive(Insertable)]
#[diesel(table_name = posts)]
pub struct NewPost<'a> {
    pub title: &'a str,
    pub body: &'a str,
}
```

With Nullable column:
```rust
#[derive(Queryable)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub hair_color: Option<String>,
}
```

With relationships (belongs_to):
```rust
#[derive(Queryable, Selectable, Identifiable, Associations)]
#[diesel(belongs_to(Book))]
#[diesel(table_name = pages)]
pub struct Page {
    pub id: i32,
    pub page_number: i32,
    pub content: String,
    pub book_id: i32,
}
```

With AsChangeset (for partial updates):
```rust
#[derive(Queryable, Identifiable, AsChangeset)]
pub struct Post {
    pub id: i32,
    pub title: String,
    pub body: String,
}
```

## Diesel CRUD Examples for SQLite

### INSERT — single record via struct
```rust
pub fn create_post(conn: &mut SqliteConnection, new_title: &str, new_body: &str) -> Post {
    let new_post = NewPost { title: new_title, body: new_body };

    diesel::insert_into(posts::table)
        .values(&new_post)
        .returning(Post::as_returning())
        .get_result(conn)
        .expect("Error saving new post")
}
```

### INSERT — column-by-column
```rust
pub fn new_author(conn: &mut SqliteConnection, author_name: &str) -> Author {
    diesel::insert_into(authors::table)
        .values(authors::name.eq(author_name))
        .returning(Author::as_returning())
        .get_result(conn)
        .expect("Error saving author")
}
```

### INSERT — multiple columns as tuple
```rust
pub fn new_page(conn: &mut SqliteConnection, page_num: i32, page_content: &str, book_id: i32) -> Page {
    diesel::insert_into(pages::table)
        .values((
            pages::page_number.eq(page_num),
            pages::content.eq(page_content),
            pages::book_id.eq(book_id),
        ))
        .returning(Page::as_returning())
        .get_result(conn)
        .expect("Error saving page")
}
```

### QUERY — list all with filter, limit
```rust
pub fn list_published_posts(conn: &mut SqliteConnection) -> Vec<Post> {
    posts
        .filter(posts::published.eq(true))
        .limit(5)
        .select(Post::as_select())
        .load(conn)
        .expect("Error loading posts")
}
```

### QUERY — find by ID (returns Option)
```rust
pub fn get_post(conn: &mut SqliteConnection, post_id: i32) -> Option<Post> {
    posts
        .find(post_id)
        .select(Post::as_select())
        .first(conn)
        .optional()
        .expect("Error fetching post")
}
```

### QUERY — find by field with filter
```rust
pub fn find_book_by_title(conn: &mut SqliteConnection, search_title: &str) -> Option<Book> {
    books
        .filter(books::title.eq(search_title))
        .select(Book::as_select())
        .first(conn)
        .optional()
        .expect("Error fetching book")
}
```

### QUERY — load all ordered
```rust
pub fn list_all_authors(conn: &mut SqliteConnection) -> Vec<Author> {
    authors
        .order(authors::name.asc())
        .select(Author::as_select())
        .load(conn)
        .expect("Error loading authors")
}
```

### QUERY — filter by enum column (IMPORTANT)
```rust
pub fn find_verified_contacts(conn: &mut SqliteConnection) -> Vec<Contact> {
    contacts
        .filter(contacts::contact_type.eq(ContactType::Email.as_str()))
        .filter(contacts::verified_at.is_not_null())
        .select(Contact::as_select())
        .load(conn)
        .expect("Error loading contacts")
}
```

### QUERY — find_by_X pattern with enum (COPY THIS for find_by_phone, find_by_email, etc.)
```rust
pub fn find_by_email(pool: &DbPool, email: &str) -> Result<Option<Self>, diesel::result::Error> {
    let mut conn = pool.get().expect("Failed to get connection");
    user_contacts::table
        .filter(
            user_contacts::contact_type.eq(ContactType::Email.as_str())
                .and(user_contacts::value.eq(email)),
        )
        .select(Self::as_select())
        .first::<Self>(&mut conn)
        .optional()
}
```

Notice: only TWO filters — the enum column + the value column. NO `user_id` filter unless the function has a `user_id` parameter.

### UPDATE — single field
```rust
pub fn publish_post(conn: &mut SqliteConnection, post_id: i32) -> Post {
    diesel::update(posts.find(post_id))
        .set(posts::published.eq(true))
        .returning(Post::as_returning())
        .get_result(conn)
        .expect("Error publishing post")
}
```

### UPDATE — multiple fields as tuple
```rust
pub fn update_post(conn: &mut SqliteConnection, post_id: i32, new_title: &str, new_body: &str) -> Post {
    diesel::update(posts.find(post_id))
        .set((posts::title.eq(new_title), posts::body.eq(new_body)))
        .returning(Post::as_returning())
        .get_result(conn)
        .expect("Error updating post")
}
```

### UPDATE — using AsChangeset struct (partial update, None = skip field)
```rust
pub fn update_post_partial(conn: &mut SqliteConnection, post_id: i32, form: PostForm) -> Post {
    diesel::update(posts::table.find(post_id))
        .set(&form)
        .returning(Post::as_returning())
        .get_result(conn)
        .expect("Error updating post")
}
```

### DELETE — by ID
```rust
pub fn delete_post(conn: &mut SqliteConnection, post_id: i32) -> usize {
    diesel::delete(posts.find(post_id))
        .execute(conn)
        .expect("Error deleting post")
}
```

### DELETE — by pattern match (LIKE)
```rust
pub fn delete_posts_by_title_pattern(conn: &mut SqliteConnection, search_pattern: &str) -> usize {
    let search_pattern = format!("%{search_pattern}%");
    diesel::delete(posts.filter(posts::title.like(search_pattern)))
        .execute(conn)
        .expect("Error deleting posts")
}
```

### RELATIONSHIP — get children of a parent (belonging_to)
```rust
pub fn get_pages_for_book(conn: &mut SqliteConnection, parent_book: &Book) -> Vec<Page> {
    Page::belonging_to(parent_book)
        .select(Page::as_select())
        .load(conn)
        .expect("Error loading pages")
}
```

### RELATIONSHIP — JOIN (inner_join)
```rust
pub fn get_pages_with_book(conn: &mut SqliteConnection) -> Vec<(Page, Book)> {
    pages::table
        .inner_join(books::table)
        .select((Page::as_select(), Book::as_select()))
        .load(conn)
        .expect("Error loading pages with books")
}
```

"#,
    );

    // ── The target struct ──────────────────────────────────────────────────
    prompt.push_str("## The Model\n\n```rust\n");
    prompt.push_str(struct_code);
    prompt.push_str("\n```\n\n");

    // ── Dependent types (enums, etc.) ──────────────────────────────────────
    if !dependent_types.is_empty() {
        prompt.push_str("## Dependent Types\n\n");
        for dep in dependent_types {
            prompt.push_str(&format!(
                "From `{}`:\n```rust\n{}\n```\n\n",
                dep.file.file_name().unwrap_or_default().to_string_lossy(),
                dep.source
            ));
        }
        prompt.push_str(
            "To filter on an enum column, use `EnumType::Variant.as_str()` against the column:\n\
            ```rust\n\
            table.filter(table::enum_column.eq(EnumType::Variant.as_str()))\n\
            ```\n\n",
        );
    }

    // ── Existing impl functions (style examples) ───────────────────────────
    let examples: Vec<&CodeBlock> = example_fns.iter().take(3).collect();
    if !examples.is_empty() {
        prompt.push_str("Existing functions in this impl block (match this style):\n```rust\n");
        for f in &examples {
            prompt.push_str(&f.source);
            prompt.push('\n');
        }
        prompt.push_str("```\n\n");
    }

    // ── Final instruction ──────────────────────────────────────────────────
    prompt.push_str("## Task\n\n");
    prompt.push_str(&format!(
        "Write the function `{}` for the model above. Return ONLY the function definition. No imports, no explanation, no markdown fences.",
        fn_name
    ));

    // Column reminder at the very end — last thing the model reads.
    if !columns.is_empty() {
        prompt.push_str(&format!(
            "\n\nReminder: the only valid columns for `{}` are: {}.",
            table_name.as_deref().unwrap_or("this table"),
            columns.join(", ")
        ));
    }

    (prompt, table_name)
}
