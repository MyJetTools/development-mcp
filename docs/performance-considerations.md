# Performance Considerations

Principles every contributor should follow by default. New PRs should
comply with them out of the box; deviations need an explicit
justification in the commit/PR description.

## 1. ArcSwap for read-mostly data

If the structure is:

- **read very often** (hot path — every publish / read message, every
  timer tick, every gRPC/HTTP request), and
- **written very rarely** (topic creation, topic deletion, init,
  periodic snapshot),

it should live behind `arc_swap::ArcSwap<Inner>`, not behind
`Mutex<Inner>` / `RwLock<Inner>`.

**Why:**
- readers take no lock at all — it's an atomic load of an `Arc`,
- writers do not block readers,
- no contention on RwLock CAS / counters with many concurrent readers,
- readers never get stalled behind a writer.

**How:**
- keep state in `Arc<Inner>`, fully immutable,
- writes are copy-on-write: clone, mutate the copy, then
  `inner.store(Arc::new(...))`,
- serialize concurrent writers with a small `Mutex<()>` (just to make
  the read-modify-write CAS not lose updates); readers do not see it,
- if `Inner` contains a "heavy" list that is often requested in full —
  cache `Arc<Vec<Arc<T>>>` next to the main container so that
  `get_all()` returns a clone of the `Arc` without allocating.

**Reference:** [src/topic_data/topics_data_list.rs](src/topic_data/topics_data_list.rs)
and `my-service-bus/src/topics/topics_list.rs` (sibling crate).

**When NOT to use ArcSwap:**
- if writes happen at a rate comparable to reads,
- if the state is mutated in-place in tiny pieces (e.g. a queue with
  push/pop on every message) — copy-on-write of the whole container is
  more expensive than the lock itself.

## 2. parking_lot::Mutex / RwLock by default

Any synchronous-only lock should be `parking_lot::Mutex` /
`parking_lot::RwLock`, not `tokio::sync::Mutex` / `tokio::sync::RwLock`
and not `std::sync::Mutex`.

**Why:**
- faster (uncontended lock — a couple of atomic ops, no syscall),
- smaller (`sizeof = 1 word`, vs several words for tokio),
- `lock()` is synchronous — no useless future / re-scheduling on every
  acquire,
- panics honestly on reentrant lock in the same thread (no UB).

**Rule:**
- critical section is purely synchronous (no `.await` while holding the
  guard) → use `parking_lot`,
- if you find yourself wanting to `.await` while holding a guard —
  **refactor first**: gather data under the lock, drop the guard, then
  `.await` over the gathered data. See
  `pages_list.rs::get_messages_amount_to_save` — it clones a
  `Vec<Arc<...>>` under the lock and iterates outside it.

## 3. tokio::sync::Mutex / RwLock — only when there is no other way

`tokio::sync::Mutex` / `RwLock` is justified **only** when it is
architecturally impossible to avoid holding a `MutexGuard` across
`.await`. That is, the critical section inherently contains an await
(the lock has to be held across network/file I/O) and there is no
cheap way to restructure the code.

**Why this is a hard limit:**
- a tokio mutex is several times more expensive than parking_lot,
- holding a lock across `.await` is almost always a sign of bad
  decomposition; the await blocks every other writer/reader for as long
  as the I/O takes and serializes the runtime,
- a `parking_lot::MutexGuard` is `!Send` → trying to drag it across
  `.await` is a compile error, which is a feature (it forces the
  refactor).

**If we still keep a tokio mutex — add a comment on the field/method
briefly explaining why parking_lot doesn't fit.**

## 4. AHash instead of std HashMap/HashSet

For in-memory maps and sets use `ahash::AHashMap` / `ahash::AHashSet`,
not `std::collections::HashMap` / `std::collections::HashSet`.

**Why:**
- the std `DefaultHasher` (SipHash) is a cryptographically strong hash;
  we don't need that for our internal keys (`topic_id`,
  `MinuteWithinYear`, `i64`, …),
- AHash is 3–10× faster on typical short string keys,
- the API is identical (`AHashMap::new()`, `insert/get/remove`, `iter`,
  …) — migration is a type swap in the import.

**Exceptions:**
- if a container can legitimately receive keys from untrusted external
  clients and hash-flooding is a threat — keep `std::HashMap` or use
  `RandomState`. We don't have such a place yet.
- if the container is serialized via serde and stable key order is
  required — use `BTreeMap` (already done in `archive_storage_list`,
  `index_by_minute_list`, etc.).

## 5. Where this is already applied (as a reference)

- ArcSwap: [src/topic_data/topics_data_list.rs](src/topic_data/topics_data_list.rs)
- parking_lot::Mutex: [src/message_pages/sub_page.rs](src/message_pages/sub_page.rs),
  [src/message_pages/pages_list.rs](src/message_pages/pages_list.rs),
  [src/archive_storage/archive_storage_list.rs](src/archive_storage/archive_storage_list.rs),
  [src/index_by_minute/update_queue.rs](src/index_by_minute/update_queue.rs),
  [src/app/prometheus_metrics/prometheus_metrics.rs](src/app/prometheus_metrics/prometheus_metrics.rs)
- parking_lot::RwLock: [src/topics_snapshot/current_snapshot.rs](src/topics_snapshot/current_snapshot.rs),
  [src/index_by_minute/index_by_minute_list.rs](src/index_by_minute/index_by_minute_list.rs)
- AHashMap / AHashSet: [src/app/prometheus_metrics/prometheus_metrics.rs](src/app/prometheus_metrics/prometheus_metrics.rs),
  [src/timers/metrics_updater.rs](src/timers/metrics_updater.rs)

## 6. No heavy CPU work under a Mutex

Especially now that we use `parking_lot::Mutex` everywhere: the guard
is a real blocking lock on a tokio worker thread. If we do something
genuinely heavy under it (compression, serialization of large
payloads, encryption), every other competing writer/reader waits on
that thread, and unrelated tasks on the same runtime stall behind it.

**Rule:** under `Mutex<X>` we only touch `X`
(`get`/`insert`/`remove`/"take a snapshot"). Any processing of the
data we got — outside the lock.

**A current suspicious spot:**
[src/message_pages/sub_page.rs::to_compressed_payload](src/message_pages/sub_page.rs)
— under the `SubPageInner` lock it builds a `CompressedPageBuilder`
and encodes every message. On large sub-pages this is tens of ms of
CPU under a lock. Better: under the lock clone
`Vec<Arc<MessageProtobufModel>>`, drop the guard, then build the
payload without the lock.

## 7. Return `Arc<Vec<...>>`, not `Vec<...>`, for read-mostly snapshots

When we already have an `ArcSwap` snapshot and a `get_all()` method
that exposes "the whole list" — return the `Arc<Vec<Arc<T>>>` that
already lives inside the inner snapshot, without copying.

**Why:**
- the reader just clones the `Arc` (one atomic inc),
- no fresh `Vec` is allocated per call,
- if the list is actually large (thousands of topics, tens of
  thousands of pages), it is a measurable difference on the hot path.

**Reference:** `TopicsDataList::get_all()` returns
`Arc<Vec<Arc<TopicData>>>`; on the caller side iterate with
`for x in vec.iter()` or `&*vec`, do not iterate by value.

## 8. No unnecessary `.clone()` on strings / payloads

In particular:
- map / set keys are already `String` — for `get` use `&str`, do not
  clone,
- message payloads (`Vec<u8>`) live as `Arc<MessageProtobufModel>` and
  travel through the system as `Arc::clone`, never as
  `Vec<u8>::clone()`,
- if an API "pass the message further" needs an owned type — wrap it
  once in an `Arc` at the boundary and pass `Arc` everywhere after.

## 9. Async I/O outside locks — parallelize explicitly

When the code does "for each topic / page — issue a network call"
(flush to blob, GC over containers, etc.), the default shape is a
serial `for x in items { do_io(x).await }`. That is safe but slower
than necessary. If order doesn't matter and the work is independent,
consider `futures::future::join_all` or `FuturesUnordered` with a
bounded concurrency.

**Rule:**
- when adding a new I/O loop, consciously decide: serial vs
  bounded-parallel,
- if parallel — always with an upper bound on in-flight calls (do not
  fire N thousand requests at once).

## 10. PR review checklist

- [ ] Is this container read-mostly? If yes — `ArcSwap`, not `RwLock`.
- [ ] Is the lock held across `.await`? If no — `parking_lot`, not
      `tokio::sync`.
- [ ] If `tokio::sync::Mutex/RwLock` is used — is there a comment
      explaining why `parking_lot` doesn't fit?
- [ ] Any `std::collections::HashMap/HashSet`? If keys are internal —
      switch to `AHashMap/AHashSet`.
- [ ] Did a `MutexGuard` end up held across `.await`? If yes —
      refactor (collect `Vec<Arc<...>>` under the lock, drop the guard,
      process outside).
- [ ] Is there heavy CPU work (compression, serialization of large
      payloads, encryption) under the lock? If yes — move it out.
- [ ] Does `get_all()` / any "give me the whole list" return
      `Arc<Vec<Arc<T>>>` from the ArcSwap snapshot, instead of cloning
      a Vec?
- [ ] No unnecessary `.clone()` of payloads / large `Vec<u8>` — passed
      through `Arc`?
- [ ] If the new code has `for x in items { do_io(x).await }` — is the
      serial mode justified, or should it be `join_all` /
      `FuturesUnordered` with a concurrency limit?

---

**Note on `panic!`:** intentionally not listed here. Our convention is
that `panic!` is acceptable when there is nothing to do with the error
— e.g. the database is unreachable on startup, no service can do
anything useful in that state. Services are expected to catch the
panic and surface it as a `FatalError`. So panic is a deliberate
operational signal, not a perf concern.
