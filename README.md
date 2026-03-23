# GitBones

A Rust CLI for git-based deployments over SSH. GitBones scaffolds hook scripts and deployment configs into your repo, syncs them to a remote bare repository, and manages file ownership and permissions across deploys.

It produces two binaries:
- **`gitbones`** — local CLI for setup and management
- **`gitbones-remote`** — server-side tool for remote operations, installed on the deployment host

## How It Works

GitBones uses a two-user deployment model:

1. A **deploy user** (default: `git`) handles SSH access and runs deployment scripts. This user has restricted sudo ability but no password login.
2. A **service user** (default: `applications`) owns the deployed files. This user has no home folder, no login, and no sudo ability — limiting attack scope.

During deployment, `gitbones-remote` temporarily changes file ownership to the deploy user so scripts can write, then hardens permissions back to the service user afterward. The sudoers configuration is strictly limited to `gitbones-remote` commands only.

## Installation

### Local (gitbones)

```sh
cargo install --git https://github.com/alexjgriffith/gitbones.git gitbones
```

### Server (gitbones-remote)

```sh
cargo install --git https://github.com/alexjgriffith/gitbones.git gitbones-remote
```

Then run the one-time server setup as root:

```sh
sudo gitbones-remote init
```

This installs a sudoers drop-in at `/etc/sudoers.d/gitbones` so the deploy user can run `gitbones-remote` without a password.

## Usage

### Initial Setup

In your project repository:

```sh
gitbones init
```

This will:
1. Create a `.bones/` folder with hooks and deployment script templates
2. Prompt for project configuration (remote name, host, permissions, etc.)
3. Add `.bones` to `.gitignore`
4. Symlink the `pre-push` hook into `.git/hooks/`
5. Create a bare repo on the remote if needed
6. Upload the `post-receive` hook to the remote

A git remote must already be configured for the deployment target:

```sh
git remote add production git@deploy.example.com:/home/git/myproject.git
```

### Syncing Configuration

After editing hooks or deployment scripts in `.bones/`:

```sh
gitbones push
```

This rsyncs `.bones/` to the remote bare repo and symlinks the hooks.

### Deploying

Just push to your deployment remote:

```sh
git push production master
```

The hook chain handles the rest:
1. **pre-push** (local) — runs `gitbones doctor --local`
2. **pre-receive** (remote) — runs `gitbones-remote doctor`, then `pre-deploy`
3. **pre-deploy** (remote) — changes worktree ownership to deploy user
4. **post-receive** (remote) — checks out latest commit to worktree
5. **deploy** (remote) — runs scripts in `.bones/deployment/` sequentially
6. **post-deploy** (remote) — hardens permissions back to service user

### Health Checks

```sh
gitbones doctor          # check local + remote
gitbones doctor --local  # check local only
```

## Configuration

`gitbones init` generates `.bones/bones.toml`:

```toml
[data]
remote_name = "production"
project_name = "myproject"
host = "deploy.example.com"
port = "22"
git_dir = "/home/git/myproject.git"
worktree = "/var/www/myproject"
branch = "master"

[permissions.defaults]
deploy = "git"
owner = "applications"
group = "www-data"
dir_mode = "750"
file_mode = "640"

[[permissions.paths]]
path = "storage"
mode = "770"
recursive = true

[[permissions.paths]]
path = "database/database.sqlite"
mode = "660"
type = "file"
```

## Project Structure

```
.bones/
├── bones.toml           # project configuration
├── deployment/
│   └── 01_*.sh          # deployment scripts (run sequentially)
└── hooks/
    ├── pre-push         # symlinked to .git/hooks/pre-push
    ├── pre-receive
    ├── pre-deploy
    ├── post-receive
    ├── deploy
    └── post-deploy
```

Hooks are written to `.bones/hooks/` once during init. After that they belong to you — edit freely. Deployment scripts in `.bones/deployment/` must be numbered (e.g. `01_install_deps.sh`, `02_build.sh`) and are always run in order.

## License

MIT
