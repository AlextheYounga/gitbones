# Symlink-Based Releases and Rollbacks

## Context

Currently, every deploy does `git checkout -f` directly into the worktree (`/var/www/lawsnipe`), overwriting files in-place. There's no way to instantly roll back if something breaks, and persistent files like `.env` and `storage/` get their permissions churned every deploy. This feature introduces Capistrano-style releases: each deploy goes into a timestamped directory, a `current` symlink points to the active release, and rollback is just a symlink swap.

No backward compatibility is needed — no one else is using gitbones yet.

## Target Directory Structure

```
/var/www/lawsnipe/
├── releases/
│   ├── 20260323_183800/
│   ├── 20260323_192100/
│   └── 20260323_201500/  ← latest
├── shared/
│   ├── .env
│   ├── storage/
│   └── node_modules/
└── current -> releases/20260323_201500/
```

Web server document root becomes `/var/www/lawsnipe/current/public`.

## New Config Fields

Add `[releases]` section to bones.toml:

```toml
[releases]
keep = 5
shared_paths = [".env", "storage", "node_modules"]
```

## New Deploy Flow

1. **pre-receive**: `sudo gitbones-remote doctor` (unchanged), then calls `pre-deploy`
2. **pre-deploy**: `sudo gitbones-remote prepare-release --config ...`
   - Creates `releases/` and `shared/` dirs if missing
   - Creates `releases/{YYYYMMDD_HHMMSS}/`
   - Chowns new release dir + shared dir to deploy user
   - Writes release name to `{git_dir}/bones/.current_release` (hook state file)
3. **post-receive**: `git checkout -f` into `releases/{timestamp}/` (reads `.current_release`), then calls `deploy`
4. **deploy**: `cd` into release dir, runs deployment scripts, then calls `sudo gitbones-remote activate-release --config ...`
   - Symlinks each shared_path from release dir → `shared/`
   - Atomically swaps `current` symlink (create tmp link, then `rename`)
   - Prunes old releases beyond `keep` count
   - Then calls `post-deploy`
5. **post-deploy**: `sudo gitbones-remote post-deploy --config ...`
   - Hardens permissions on the release dir pointed to by `current` + `shared/`

If a deployment script fails at step 4, `current` still points to the previous release. The site stays up.

## New Commands

### gitbones-remote (server-side)

| Command | Description |
|---------|-------------|
| `prepare-release --config` | Create release dir, chown, write state file |
| `activate-release --config` | Symlink shared paths, swap `current`, prune old releases |
| `rollback --config` | Re-point `current` to previous release |

### gitbones (local CLI)

| Command | Description |
|---------|-------------|
| `rollback` | SSH in, run `sudo gitbones-remote rollback` |

## Implementation Order

### Step 1: Config structs (both crates)
Add `Releases` struct and `releases` field to `BonesConfig`.

**Files:**
- `crates/gitbones/src/config.rs`
- `crates/gitbones-remote/src/config.rs`

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Releases {
    #[serde(default = "default_keep")]
    pub keep: u32,
    #[serde(default)]
    pub shared_paths: Vec<String>,
}

fn default_keep() -> u32 { 5 }
```

Add to `BonesConfig`:
```rust
#[serde(default)]
pub releases: Option<Releases>,
```

### Step 2: Refactor permissions.rs
Extract `harden` logic so it can accept an arbitrary root path instead of always using `worktree`. This allows hardening just the current release + shared dir.

**File:** `crates/gitbones-remote/src/permissions.rs`

- Rename existing `harden(cfg)` → `harden_paths(cfg, paths: &[&str])`
- Add `harden_release(cfg)` that resolves `current` target + `shared/` and calls `harden_paths`
- Same for `chown_to_deploy_user` — add variant that chowns specific dirs

### Step 3: New remote commands
**New files:**
- `crates/gitbones-remote/src/commands/prepare_release.rs`
- `crates/gitbones-remote/src/commands/activate_release.rs`
- `crates/gitbones-remote/src/commands/rollback.rs`

**Modified:** `crates/gitbones-remote/src/commands/mod.rs` — register new subcommands

#### prepare_release.rs
1. Load config
2. Create `{worktree}/releases/` and `{worktree}/shared/` if missing
3. Generate timestamp: `chrono::Local::now().format("%Y%m%d_%H%M%S")`
4. Create `{worktree}/releases/{timestamp}/`
5. Chown new release dir + shared dir to deploy user
6. Write timestamp to `{git_dir}/bones/.current_release`

Requires adding `chrono` dep to `crates/gitbones-remote/Cargo.toml`.
The `--config` arg gives us `bones.toml` path. Derive `git_dir` as the parent of `bones/bones.toml`.

#### activate_release.rs
1. Load config, read release name from `.current_release`
2. For each `shared_paths` entry:
   - If path exists in release dir but not in shared: move it to shared (first deploy seeds shared from checkout)
   - If path doesn't exist in shared: create it (mkdir for dirs, touch for files)
   - Remove path from release dir, create symlink `release/{path}` → `../../shared/{path}`
3. Atomic symlink swap: `symlink(target, "current.tmp")` then `rename("current.tmp", "current")`
4. Prune: list `releases/`, sort, remove oldest beyond `keep` (never remove what `current` points to)
5. Clean up `.current_release` state file

#### rollback.rs
1. Load config
2. Read `current` symlink target to get active release name
3. List `releases/`, sort, find the one before current
4. Atomic symlink swap to previous release
5. Print old → new release names

### Step 4: Update hook templates
**Files in `kit/hooks/`:**

**pre-deploy** — replace `sudo gitbones-remote pre-deploy` with `sudo gitbones-remote prepare-release`

**post-receive** — read `.current_release`, checkout into that release dir:
```bash
RELEASE_DIR=$(cat "$GIT_DIR/bones/.current_release")
RELEASE_PATH="$WORKTREE/releases/$RELEASE_DIR"
git --work-tree="$RELEASE_PATH" --git-dir="$GIT_DIR" checkout -f "$BRANCH"
```

**deploy** — cd into release dir instead of worktree, call `activate-release` after scripts succeed:
```bash
RELEASE_DIR=$(cat "$GIT_DIR/bones/.current_release")
RELEASE_PATH="$WORKTREE/releases/$RELEASE_DIR"
cd "$RELEASE_PATH"
# ... run deployment scripts ...
sudo gitbones-remote activate-release --config "$BONES_TOML"
```

**post-deploy** — unchanged (the remote command internally resolves `current` → release dir)

### Step 5: Local rollback command
**New file:** `crates/gitbones/src/commands/rollback.rs`

Same pattern as `redeploy.rs`: load config, SSH in, `stream_cmd` to run `sudo gitbones-remote rollback --config ...`.

**Modified:** `crates/gitbones/src/commands/mod.rs` — add `Rollback` variant

### Step 6: Update bones.toml template and prompts
- `kit/bones.toml` — add `[releases]` section
- `crates/gitbones/src/prompts.rs` — add prompts for `keep` and `shared_paths` during `gitbones init`

## Verification

1. `cargo check` — both crates compile
2. `cargo build --release` — build both binaries
3. Deploy `gitbones-remote` to server, run `gitbones push` to sync new hooks
4. Update `bones.toml` with `[releases]` section, run `gitbones push`
5. Update nginx document root to `/var/www/lawsnipe/current/public`, reload nginx
6. Move `.env` and `storage/` into `/var/www/lawsnipe/shared/` manually (first time only)
7. `git push production master` — verify:
   - Release dir created in `releases/`
   - `current` symlink points to new release
   - Shared paths are symlinked into release dir
   - Site works
8. Push again — verify pruning keeps only `keep` releases
9. `gitbones rollback` — verify `current` swaps to previous release, site serves old code
10. `gitbones redeploy` — verify it works with the new release flow
