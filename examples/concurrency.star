# Concurrency and Parallelism in Star

# ── Threads ─────────────────────────────────────────────

fn demo_threads() =
  do
    println("=== Threads ===")

    # spawn_join: spawn a thread and wait for result
    let result = spawn_join(fn() => 21 * 2)
    debug(result)
  end

# ── Mutex ───────────────────────────────────────────────

fn demo_mutex() =
  do
    println("")
    println("=== Mutex ===")

    let counter = mutex_new(0)
    let val = mutex_lock(counter)
    println("Mutex value: #{val}")
  end

# ── RwLock ──────────────────────────────────────────────

fn demo_rwlock() =
  do
    println("")
    println("=== RwLock ===")

    let data = rwlock_new("hello")
    let val = rwlock_read(data)
    println("RwLock read: #{val}")
  end

# ── Atomics ─────────────────────────────────────────────

fn demo_atomics() =
  do
    println("")
    println("=== Atomics ===")

    let counter = atomic_new(0)
    atomic_add(counter, 5)
    atomic_add(counter, 3)
    let val = atomic_get(counter)
    println("Atomic counter: #{val}")

    atomic_set(counter, 100)
    println("After set: #{atomic_get(counter)}")
  end

# ── Parallel map ────────────────────────────────────────

fn demo_parallel() =
  do
    println("")
    println("=== Parallel Map ===")

    let numbers = [1, 2, 3, 4, 5]
    let doubled = parallel_map(numbers, fn(x) => x * 2)
    debug(doubled)
  end

# ── Main ────────────────────────────────────────────────

fn main() =
  do
    demo_threads()
    demo_mutex()
    demo_rwlock()
    demo_atomics()
    demo_parallel()
    println("")
    println("All concurrency demos complete!")
  end
