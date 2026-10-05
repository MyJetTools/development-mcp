# Mappers Pattern

Mappers live in `mappers/` — one file per domain entity.
One file = conversions for one type (e.g. `posts.rs` maps all post-related types).

## Always `impl Into<Target> for Source` — never standalone functions

```rust
// mappers/posts.rs
use crate::posts_grpc::*;
use crate::postgres::*;

impl Into<PostGrpcModel> for PostDto {
    fn into(self) -> PostGrpcModel {
        PostGrpcModel {
            creator_id: self.creator_id,
            id: self.id,
            publish_from: self.publish_from.unix_microseconds,  // ← DateTimeAsMicroseconds → i64
            likes_amount: self.likes_amount.unwrap_or_default(),  // ← Option → default
        }
    }
}
```

## Usage in flows/handlers:
```rust
// Clean — type system handles conversion
let grpc_model: PostGrpcModel = post_dto.into();
let models: Vec<PostGrpcModel> = dtos.into_iter().map(|x| x.into()).collect();
```

## Rules:
> **WHY:** `impl Into` is idiomatic Rust. The compiler applies the conversion automatically
> wherever the target type is expected. Standalone functions require explicit calls everywhere
> and pollute the namespace. `Into` keeps flows/handlers clean: `dto.into()` instead of `map_dto_to_grpc(dto)`.

- `impl Into<Target> for Source` — ALWAYS, never `fn map_post_to_grpc(dto: PostDto) -> PostGrpcModel`
- One mapper file per source entity
- `Option<T>` → `.unwrap_or_default()` in the mapper — when NULL in DB has clear business semantics (NULL likes = 0 likes). This is domain logic expressed in the mapper.
- `DateTimeAsMicroseconds` → `i64`: use `.unix_microseconds` field directly
- NEVER put mapping logic in flows, scripts, or handlers — always in `mappers/`
