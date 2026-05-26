use crate::code_extractor::CodeBlock;

/// Extract the table name from a Diesel struct's `#[diesel(table_name = X)]` attribute.
pub fn extract_table_name(struct_code: &str) -> Option<String> {
    let start = struct_code.find("table_name = ")?;
    let rest = &struct_code[start + "table_name = ".len()..];
    let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')?;
    Some(rest[..end].to_string())
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
    let mut prompt = String::new();

    // ── Role and task ──────────────────────────────────────────────────────
    prompt.push_str(
        r#"You are a Rust expert writing Diesel ORM functions for SQLite.

Write ONLY the function body. Do NOT include imports — they are added automatically.
Return ONLY the function definition. No explanation.

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

## Diesel Schema Example (table! macro)

```rust
diesel::table! {
    posts (id) {
        id -> Integer,
        title -> Text,
        body -> Text,
        published -> Bool,
    }
}
```

## Diesel Model Struct Examples

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
pub fn create_post(conn: &mut SqliteConnection, title: &str, body: &str) -> Post {
    let new_post = NewPost { title, body };

    diesel::insert_into(posts::table)
        .values(&new_post)
        .returning(Post::as_returning())
        .get_result(conn)
        .expect("Error saving new post")
}
```

### INSERT — column-by-column
```rust
pub fn new_author(conn: &mut SqliteConnection, name: &str) -> Author {
    diesel::insert_into(authors::table)
        .values(authors::name.eq(name))
        .returning(Author::as_returning())
        .get_result(conn)
        .expect("Error saving author")
}
```

### INSERT — multiple columns as tuple
```rust
pub fn new_page(conn: &mut SqliteConnection, page_number: i32, content: &str, book_id: i32) -> Page {
    diesel::insert_into(pages::table)
        .values((
            pages::page_number.eq(page_number),
            pages::content.eq(content),
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
        .filter(published.eq(true))
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
pub fn find_book_by_title(conn: &mut SqliteConnection, title: &str) -> Option<Book> {
    books
        .filter(title.eq(title))
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
        .order(name.asc())
        .select(Author::as_select())
        .load(conn)
        .expect("Error loading authors")
}
```

### UPDATE — single field
```rust
pub fn publish_post(conn: &mut SqliteConnection, id: i32) -> Post {
    diesel::update(posts.find(id))
        .set(published.eq(true))
        .returning(Post::as_returning())
        .get_result(conn)
        .expect("Error publishing post")
}
```

### UPDATE — multiple fields as tuple
```rust
pub fn update_post(conn: &mut SqliteConnection, id: i32, new_title: &str, new_body: &str) -> Post {
    diesel::update(posts.find(id))
        .set((title.eq(new_title), body.eq(new_body)))
        .returning(Post::as_returning())
        .get_result(conn)
        .expect("Error updating post")
}
```

### UPDATE — using AsChangeset struct (partial update, None = skip field)
```rust
pub fn update_post_partial(conn: &mut SqliteConnection, id: i32, form: PostForm) -> Post {
    diesel::update(posts::table.find(id))
        .set(&form)
        .returning(Post::as_returning())
        .get_result(conn)
        .expect("Error updating post")
}
```

### DELETE — by ID
```rust
pub fn delete_post(conn: &mut SqliteConnection, id: i32) -> usize {
    diesel::delete(posts.find(id))
        .execute(conn)
        .expect("Error deleting post")
}
```

### DELETE — by pattern match (LIKE)
```rust
pub fn delete_posts_by_title_pattern(conn: &mut SqliteConnection, pattern: &str) -> usize {
    let pattern = format!("%{pattern}%");
    diesel::delete(posts.filter(title.like(pattern)))
        .execute(conn)
        .expect("Error deleting posts")
}
```

### RELATIONSHIP — get children of a parent (belonging_to)
```rust
pub fn get_pages_for_book(conn: &mut SqliteConnection, book: &Book) -> Vec<Page> {
    Page::belonging_to(book)
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

    (prompt, table_name)
}
