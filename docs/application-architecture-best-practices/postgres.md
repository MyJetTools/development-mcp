# DB / Postgres Pattern

```rust
// postgres/likes_repo.rs
service_sdk::macros::use_my_postgres!();  // ← ALWAYS first

pub const TABLE_NAME: &str = "likes";
pub const PK_NAME: &str = "likes_pk";

pub struct LikesRepo {
    postgres: MyPostgres,  // ← private field
}

impl LikesRepo {
    pub async fn new(settings_reader: Arc<SettingsReader>) -> Self {
        let postgres = MyPostgres::from_settings(APP_NAME, settings_reader)
            .with_table_schema_verification::<LikeDto>(TABLE_NAME, Some(PK_NAME.into()))
            .build()
            .await;
        Self { postgres }
    }
}
```

## Write Strategy — CRITICAL

**DEFAULT:** `insert_or_update_db_entity` — for all create and update operations.
Reason: idempotent = safe retries.

**EXCEPTION:** `insert_db_entity_if_not_exists` — ONLY for registration/unique entity creation
where the business answer is "already exists or not".

**NEVER:** raw `insert_db_entity` for regular writes — no duplicate protection on retries.

> Both `insert_or_update_db_entity` and `bulk_insert_or_update_db_entity` take `UpdateConflictType` as the **second positional argument**. The standard value is `UpdateConflictType::OnPrimaryKeyConstraint(PK_NAME.into())`. Import the type: `use service_sdk::my_postgres::UpdateConflictType;`.

## Update Pattern — Read → Modify → Write

```rust
// ✅ CORRECT
let mut entity = self.postgres
    .with_retries(3, Duration::from_secs(1))
    .query_single_row(TABLE_NAME, Some(&where_model), Some(ctx))
    .await
    .expect("entities: query_single_row get failed");
entity.status = NewStatus;
entity.updated_at = DateTimeAsMicroseconds::now();
self.postgres
    .with_retries(3, Duration::from_secs(1))
    .insert_or_update_db_entity(
        TABLE_NAME,
        UpdateConflictType::OnPrimaryKeyConstraint(PK_NAME.into()),
        &entity,
        Some(ctx),
    )
    .await
    .expect("entities: insert_or_update_db_entity failed");

// ❌ WRONG — partial update
// UPDATE SET field=value WHERE id=x  ← not retry-safe

// ❌ WRONG — no retries on write
self.postgres
    .insert_or_update_db_entity(
        TABLE_NAME,
        UpdateConflictType::OnPrimaryKeyConstraint(PK_NAME.into()),
        &entity,
        Some(ctx),
    )
    .await
    .expect("...");
```

## Retries — always use with_retries

```rust
// ✅ CORRECT — read
self.postgres
    .with_retries(3, Duration::from_secs(1))
    .query_single_row(TABLE_NAME, Some(&where_model), Some(ctx))
    .await
    .expect("chats: query_single_row get_by_participants failed");

// ✅ CORRECT — write
self.postgres
    .with_retries(3, Duration::from_secs(1))
    .insert_or_update_db_entity(
        TABLE_NAME,
        UpdateConflictType::OnPrimaryKeyConstraint(PK_NAME.into()),
        &entity,
        Some(ctx),
    )
    .await
    .expect("chats: insert_or_update_db_entity upsert failed");

// ✅ CORRECT — bulk write
self.postgres
    .with_retries(3, Duration::from_secs(1))
    .bulk_insert_or_update_db_entity(TABLE_NAME, UpdateConflictType::OnPrimaryKeyConstraint(PK_NAME.into()), items, Some(ctx))
    .await
    .expect("likes: bulk_insert_or_update_db_entity failed");

// ❌ WRONG — no retries on any operation
self.postgres
    .query_single_row(TABLE_NAME, Some(&where_model), Some(ctx))
    .await
    .expect("...");
```

**ALWAYS** `.with_retries(3, Duration::from_secs(1))` before every DB operation.
**ALWAYS** pass telemetry context `Some(ctx)` — everywhere, always.
**NEVER** pass `None` for telemetry context.

## Error Handling in DB / Postgres

> **WHY:** `MyHttpServer`, `GrpcServer`, `Postgres` — all implement retries internally.
> If an IO error reaches your code, all retry attempts are exhausted — there is nothing to do.
> Panicking is correct: the server logs it, returns 500/INTERNAL, and keeps running.
> Hiding the error with `?` or `unwrap_or` means continuing in an undefined state.

```rust
// ✅ CORRECT — read with retries + telemetry
self.postgres
    .with_retries(3, Duration::from_secs(1))
    .query_single_row(TABLE_NAME, Some(&where_model), Some(ctx))
    .await
    .expect("likes: query_single_row get_count_by_object failed");
    //       ↑ format: "{table}: {operation} failed"

// ✅ CORRECT — write with retries + telemetry
self.postgres
    .with_retries(3, Duration::from_secs(1))
    .insert_or_update_db_entity(
        TABLE_NAME,
        UpdateConflictType::OnPrimaryKeyConstraint(PK_NAME.into()),
        &entity,
        Some(ctx),
    )
    .await
    .expect("likes: insert_or_update_db_entity upsert failed");

// ❌ WRONG — propagating IO error up the call stack
let result = self.postgres.query_single_row(...).await?;

// ❌ WRONG — silently swallowing the error
let result = self.postgres.query_single_row(...).await.unwrap_or_default();

// ❌ WRONG — no retries on write
self.postgres.insert_or_update_db_entity(
    TABLE_NAME,
    UpdateConflictType::OnPrimaryKeyConstraint(PK_NAME.into()),
    &entity,
    Some(ctx),
).await.expect("...");
```

**NEVER** return `Result` from repo methods — only `.expect()`.
**ALWAYS** `.expect("{table}: {operation} failed")` — table name + method name in the message.

## Repo Methods — always `pub async fn`, never return Result

```rust
// ✅ CORRECT
pub async fn bulk_insert(&self, items: &[LikeDto], ctx: &MyTelemetryContext) {
    self.postgres
        .with_retries(3, Duration::from_secs(1))
        .bulk_insert_or_update_db_entity(
            TABLE_NAME,
            UpdateConflictType::OnPrimaryKeyConstraint(PK_NAME.into()),
            items,
            Some(ctx),
        )
        .await
        .expect("likes: bulk_insert_or_update_db_entity failed");
}

// ❌ WRONG — no retries, no ctx, returns Result
pub async fn bulk_insert(&self, items: &[LikeDto]) -> Result<(), Error>
```

## DTO Structure

```rust
// postgres/dto.rs
service_sdk::macros::use_my_postgres!();  // ← ALWAYS first, even in dto files

#[derive(SelectDbEntity, InsertDbEntity, UpdateDbEntity, Debug, TableSchema)]
pub struct LikeDto {
    #[primary_key(0)]
    #[generate_where_model("DeleteLikeWhereModel")]  // ← generate WhereModel from PK fields
    #[db_index(id:0, index_name:"like_by_object_id_idx", is_unique:true, order:"ASC")]
    pub tp: i32,

    #[primary_key(1)]
    #[generate_where_model("DeleteLikeWhereModel")]
    pub user_id: String,

    #[sql_type("timestamp")]
    pub moment: DateTimeAsMicroseconds,  // ← ALWAYS DateTimeAsMicroseconds, ALWAYS timestamp
}
```

Derive all four when table supports CRUD: `SelectDbEntity + InsertDbEntity + UpdateDbEntity + TableSchema`.

`#[generate_where_model]` on PK fields — preferred over manual WhereModel struct when types match.

## Where Models

Separate `WhereDbModel` struct when:
- Type differs from DTO (e.g. `Vec<T>` for IN queries)
- Synthetic fields not in table
- Operators (`>`, `<`, `LIKE`)
- Optional range filters

```rust
// Vec = IN ($1, $2, ...)
#[derive(WhereDbModel)]
pub struct GetByStatusesWhere {
    pub status: Vec<i32>,
}

// Use &'s str not String — zero-copy
#[derive(WhereDbModel)]
pub struct GetByUserIdWhereModel<'s> {
    pub tp: i32,
    pub user_id: &'s str,  // ← &str not String
}
```

**Naming:** `{Description}WhereModel`

## All DTOs and WhereModels in one dto.rs per repo

**NEVER** separate files for each DTO.
**ALWAYS** one `dto.rs` per repo file, all related structs inside.

## Datetime — always DateTimeAsMicroseconds

```rust
// ✅ CORRECT
#[sql_type("timestamp")]
pub created_at: DateTimeAsMicroseconds,

// ❌ WRONG
pub created_at: i64,  // chrono, SystemTime, etc.
```
