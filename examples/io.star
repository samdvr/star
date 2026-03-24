# Star I/O and file system showcase
# Demonstrates file operations, paths, environment, and process control

fn main() =
  do
    println("=== Star I/O & File System ===")
    println()

    # ── Path Operations ─────────────────────────────────
    println("--- Path Operations ---")
    let p = "/usr/local/bin/star"
    println("path:      #{p}")
    println("parent:    #{unwrap(path_parent(p))}")
    println("filename:  #{unwrap(path_filename(p))}")
    println("stem:      #{unwrap(path_stem(p))}")

    let src = "src/main.star"
    println("extension: #{unwrap(path_extension(src))}")
    println("join:      #{path_join("/usr", "local")}")
    println("absolute:  #{path_is_absolute(p)}")
    println("relative:  #{path_is_relative(src)}")
    println()

    # ── Environment ─────────────────────────────────────
    println("--- Environment ---")
    let home = env_get("HOME")
    println("HOME: #{unwrap_or(home, "(not set)")}")

    let user = env_get("USER")
    println("USER: #{unwrap_or(user, "(not set)")}")

    # Get current directory
    let cwd = current_dir()
    println("cwd: #{unwrap(cwd)}")

    # Get command-line args
    let a = args()
    println("args: #{a}")
    println()

    # ── File Write & Read ───────────────────────────────
    println("--- File Write & Read ---")
    let tmp_dir = "/tmp/star_io_test"
    let _ = create_dir_all(tmp_dir)

    let test_file = path_join(tmp_dir, "hello.txt")
    let _ = write_file(test_file, "Hello from Star!\nLine 2\nLine 3")
    println("wrote: #{test_file}")

    # Check existence
    println("exists: #{file_exists(test_file)}")

    # Read it back
    let content = read_file(test_file)
    println("content: #{unwrap(content)}")

    # Read as lines
    let file_lines = read_lines(test_file)
    println("lines: #{unwrap(file_lines)}")

    # File size
    let size = file_size(test_file)
    println("size: #{unwrap(size)} bytes")
    println()

    # ── Append ──────────────────────────────────────────
    println("--- Append ---")
    let _ = append_file(test_file, "\nLine 4 (appended)")
    let updated = read_file(test_file)
    println("after append:")
    println(unwrap(updated))
    println()

    # ── Copy & Rename ───────────────────────────────────
    println("--- Copy & Rename ---")
    let copy_path = path_join(tmp_dir, "copy.txt")
    let _ = copy_file(test_file, copy_path)
    println("copied to: #{copy_path}")
    println("copy exists: #{file_exists(copy_path)}")

    let renamed_path = path_join(tmp_dir, "renamed.txt")
    let _ = rename_file(copy_path, renamed_path)
    println("renamed to: #{renamed_path}")
    println("old exists: #{file_exists(copy_path)}")
    println("new exists: #{file_exists(renamed_path)}")
    println()

    # ── Directory Operations ────────────────────────────
    println("--- Directory Operations ---")
    let sub = path_join(tmp_dir, "subdir")
    let _ = create_dir(sub)
    println("created: #{sub}")
    println("is dir: #{dir_exists(sub)}")

    # Write some files in the test dir
    let _ = write_file(path_join(sub, "a.txt"), "a")
    let _ = write_file(path_join(sub, "b.txt"), "b")

    let entries = list_dir(sub)
    let sorted_entries = sort(unwrap(entries))
    println("ls subdir: #{sorted_entries}")
    println()

    # ── Process / Command ───────────────────────────────
    println("--- Process ---")
    let result = command_output("echo 'hello from shell'")
    match result
    | Ok(output) =>
      do
        debug(output)
      end
    | Err(e) => println("error: #{e}")
    end

    # Simple command execution (just exit code)
    let status = command("echo 'test'")
    println("exit code: #{unwrap(status)}")
    println()

    # ── Cleanup ─────────────────────────────────────────
    println("--- Cleanup ---")
    let _ = delete_dir(tmp_dir)
    println("deleted: #{tmp_dir}")
    println("exists after delete: #{dir_exists(tmp_dir)}")
    println()

    println("=== Done! ===")
  end
