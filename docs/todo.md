# GitBones TODO

## Phase 1: Workspace & Scaffolding
- [ ] Convert to Cargo workspace with `crates/gitbones` and `crates/gitbones-remote`
- [ ] Set up clap CLI skeleton for both binaries (subcommands, help text)
- [ ] Wire up `version` command for both binaries

## Phase 2: Config & Embedded Assets
- [ ] Define `bones.toml` serde structs in `gitbones/src/config.rs`
- [ ] Implement load/save for local config (`.bones/bones.toml`)
- [ ] Set up `rust-embed` pointing at `kit/` in `gitbones/src/embedded.rs`
- [ ] Write scaffold extraction: create `.bones/` directory tree from embedded assets
- [ ] Write the kit hook scripts (pre-push, pre-receive, pre-deploy, post-receive, deploy, post-deploy)
- [ ] Write a starter deployment script for kit (`01_run_deployment_concerns.sh`)

## Phase 3: gitbones init
- [ ] Implement `prompts.rs` using inquire (collect all bones.toml fields with defaults)
- [ ] Implement `git.rs` (read remote URL from git2, validate repo state)
- [ ] Implement init command orchestration:
  - [ ] Explain what will happen, confirm with user
  - [ ] Extract scaffold to `.bones/`
  - [ ] Update `.gitignore` to include `.bones`
  - [ ] Load existing config or run prompts for new config
  - [ ] Save config to `.bones/bones.toml`
- [ ] Symlink `.bones/hooks/pre-push` to `.git/hooks/pre-push`

## Phase 4: SSH & Remote Setup (gitbones init, continued)
- [ ] Implement `ssh.rs` (openssh session from host/port/deploy_user in config)
- [ ] Create bare repo on remote if it doesn't exist
- [ ] Upload post-receive hook to remote bare repo

## Phase 5: gitbones push
- [ ] Implement rsync of `.bones/` to `{git_dir}/bones/` on remote
- [ ] Delete sample hooks from remote bare repo `{git_dir}/hooks/`
- [ ] Symlink `{git_dir}/bones/hooks/*` to `{git_dir}/hooks/` on remote

## Phase 6: gitbones doctor
- [ ] Local checks:
  - [ ] `.bones/` folder structure is valid
  - [ ] Deployment scripts follow naming convention
  - [ ] `pre-push` hook is symlinked to `.git/hooks/pre-push`
- [ ] Remote checks (over SSH):
  - [ ] `gitbones-remote` is globally available on remote
  - [ ] `{git_dir}/bones/` exists on remote
  - [ ] `{git_dir}/bones/hooks/` entries match `{git_dir}/hooks/` symlinks

## Phase 7: gitbones-remote init
- [ ] Define `bones.toml` serde structs in `gitbones-remote/src/config.rs`
- [ ] Check that command is run as root/sudo
- [ ] Write `/etc/sudoers.d/gitbones` drop-in file
- [ ] Validate with `visudo -c`

## Phase 8: gitbones-remote doctor
- [ ] Check `gitbones-remote` can run without password (sudo -n)
- [ ] Check `gitbones-remote` is globally available (which/command -v)

## Phase 9: gitbones-remote pre-deploy & post-deploy
- [ ] Implement `config.rs` for remote (discover `bones.toml` relative to bare repo)
- [ ] `pre-deploy`: chown worktree to deploy user
- [ ] `post-deploy`: implement `permissions.rs`
  - [ ] Apply default ownership (service_user:service_group)
  - [ ] Apply default dir_mode and file_mode
  - [ ] Apply path overrides (recursive, type=dir, type=file)

## Phase 10: End-to-end testing
- [ ] Full flow: init -> push -> git push -> deploy cycle
- [ ] Verify permissions are correct after post-deploy
- [ ] Test with non-default port and host
- [ ] Test re-running init on an existing project (idempotency)
- [ ] Test doctor catches misconfigurations
