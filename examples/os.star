# Star OS & Environment Interaction showcase
# Demonstrates process management, file metadata, permissions, and symlinks

fn main() =
  do
    println("=== Star OS & Environment ===")
    println()

    # ── Process Info ─────────────────────────────────────
    println("--- Process Info ---")
    let pid = process_id()
    println("process id: #{pid}")

    let exe = exe_path()
    println("exe path: #{unwrap_or(exe, "(unknown)")}")

    let tmp = temp_dir()
    println("temp dir: #{tmp}")
    println()

    # ── Command with Args (no shell) ────────────────────
    println("--- Command with Args ---")
    let result = command_with_args_output("echo", ["hello", "from", "star"])
    match result
    | Ok(output) =>
      do
        let (stdout, _stderr, code) = output
        println("stdout: #{trim(stdout)}")
        println("exit code: #{code}")
      end
    | Err(e) => println("error: #{e}")
    end
    println()

    # ── Command with Stdin ──────────────────────────────
    println("--- Command with Stdin ---")
    let piped = command_with_stdin("cat", "data piped to cat!")
    match piped
    | Ok(output) =>
      do
        let (stdout, _stderr, _code) = output
        println("piped output: #{trim(stdout)}")
      end
    | Err(e) => println("error: #{e}")
    end
    println()

    # ── File Metadata ───────────────────────────────────
    println("--- File Metadata ---")
    let test_dir = "/tmp/star_os_test"
    let _ = create_dir_all(test_dir)
    let test_file = path_join(test_dir, "meta.txt")
    let _ = write_file(test_file, "hello metadata")

    println("is_file: #{is_file(test_file)}")
    println("is_dir:  #{is_dir(test_dir)}")
    println("is_dir (file): #{is_dir(test_file)}")

    let modified = file_modified(test_file)
    println("modified (unix ts): #{unwrap(modified)}")

    let created = file_created(test_file)
    println("created (unix ts):  #{unwrap(created)}")
    println()

    # ── Permissions ─────────────────────────────────────
    println("--- Permissions ---")
    let ro = file_readonly(test_file)
    println("readonly: #{unwrap(ro)}")

    let _ = set_readonly(test_file, true)
    let ro2 = file_readonly(test_file)
    println("after set_readonly(true): #{unwrap(ro2)}")

    # Restore writable so we can clean up
    let _ = set_readonly(test_file, false)
    println("restored writable")
    println()

    # ── Symlinks ────────────────────────────────────────
    println("--- Symlinks ---")
    let link_path = path_join(test_dir, "link.txt")
    let _ = symlink(test_file, link_path)
    println("created symlink: #{link_path}")
    println("is_symlink: #{is_symlink(link_path)}")

    let target = read_link(link_path)
    println("link target: #{unwrap(target)}")
    println()

    # ── Canonicalize ────────────────────────────────────
    println("--- Canonicalize ---")
    let canon = canonicalize(test_file)
    println("canonical: #{unwrap(canon)}")
    println()

    # ── Cleanup ─────────────────────────────────────────
    println("--- Cleanup ---")
    let _ = delete_dir(test_dir)
    println("deleted: #{test_dir}")
    println("exists: #{dir_exists(test_dir)}")
    println()

    println("=== Done! ===")
  end
