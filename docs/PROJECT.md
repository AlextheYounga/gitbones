# GitBones v3

A Rust CLI that compiles into a single binary, containing embeds of boilerplate scripts along with other git remote helpers. It produces two executables: `gitbones` (local CLI for setup and management) and `gitbones-remote` (server-side tool for remote operations, installed on the deployment host).

## Deployment Methodology
We have an SSH deployment user (normally `git`) that handles deployment concerns. This user has a home folder, restricted sudo ability, but no password login. We also have a service user: `applications`. This user has no home folder, no login, and no sudo ability. This is ultimately who we want to own our project files to limit attack scope.

### Common Problems
- Shared groups have too many logic traps. My apps should not have 660 or 770 permissions on all files so that a `git` user can have read/write.
- I don't like ACLs; they're far too opaque.
- Setting up inotify systems are cumbersome.

### Proposed Solution of This Project
We create a `gitbones-remote` executable that does not require a password and allows it to change ownership to a deploy user and harden back the permissions based on what is configured under `permissions` in bones.toml. Running `gitbones-remote init` on the remote (as sudo) installs a drop-in file at `/etc/sudoers.d/gitbones`, allowing the `git` user to run gitbones-remote without password.

## Bones Scaffolding
.bones  
├── bones.toml  
├── deployment  
│   ├── 01_run_deployment_concerns.sh
│   └── 02_permissions_lockup.sh (example)  
└── hooks  
    ├── deploy  
    ├── post-deploy  
    ├── post-receive
	├── pre-deploy    
    ├── pre-push  
    └── pre-receive  

### Bones Toml
This stores crucial data we will need and is collected on running `gitbones init` via user prompts.  
Collects the following project information from the user:  
- `remote_name`: str (production, staging, etc.)  
- `project_name`: str  
- `host`: str
- `port`: str
- `git_dir`: str (defaults to `/home/git/{project_name}.git`)  
- `worktree`: str (defaults to `/var/www/{project_name}`)  
- `branch`: str (defaults to master)  

Then we ask permissions questions:  
- `deploy_user`: str (defaults to "git")  
- `service_user`: str (defaults to "applications" - a service user who has final ownership of the site)  
- `service_group`: str (defaults to www-data)  

Example `bones.toml`:
```toml
[data]
remote_name = "production"
project_name = "lawsnipe"
host = "deploy.example.com"
port = "22"

git_dir = "/home/git/lawsnipe.git"  
worktree = "/var/www/lawsnipe"  
branch = "master"  

# These are the permissions that ultimately get applied to every file post-deployment.  
[permissions.defaults]  
deploy = "git"  
owner = "applications"  
group = "www-data"  
dir_mode   = "750"  
file_mode  = "640"  

# Path overrides for fine-grained permission control.
# recursive = true: applies 'mode' to the directory and all files/directories under it.
# type = "dir": applies 'mode' to just that directory.
# type = "file": applies 'mode' to just that file.
[[permissions.paths]]
path      = "storage"
mode      = "770"
recursive = true  

[[permissions.paths]]  
path      = "bootstrap/cache"  
mode      = "770"  
recursive = true  

[[permissions.paths]]  
path      = "database"  
mode      = "770"  
type      = "dir"  

[[permissions.paths]]  
path      = "database/database.sqlite"  
mode      = "660"  
type      = "file"  
```

### Hooks
Hooks are static shell scripts embedded in the `gitbones` binary. They are written to `.bones/hooks/` once during `gitbones init`. After that, they belong to the user and can be edited freely. They are synced to the remote bare repo via `gitbones push`.

- `pre-push` => Local hook, symlinked to `.git/hooks/pre-push`. This checks to see if we are pushing to our gitbones designated remote. If so, then we run `gitbones doctor` and we fail if the doctor command expresses any warning or errors.  
- `pre-receive` => This runs `gitbones-remote doctor` and fails if there are any warnings or errors. Then we call `pre-deploy`.
- `pre-deploy` => This runs `gitbones-remote pre-deploy`, which changes the permissions of the worktree to be owned by the ssh_user, which is necessary in order to run deployment scripts from this user. `gitbones-remote` will be allowlisted in the sudoers file. 
- `post-receive` => Update our git worktree to the latest commit (standard git deployment flow). Then it will call our custom git-hook, `deploy`.  
- `deploy` => Our custom meta caller. This will scan our `{project_name}.git/bones/deployment` folder and run these scripts sequentially. 
- `post-deploy` => Runs `gitbones-remote post-deploy`, which sets permissions back to our service user as detailed in bones.toml

### Deployment Folder
This folder stores deployment scripts to be called by `deploy`. Files in this folder must be ordered sequentially like `01_run_deployment_concerns.sh` and `02_lockup_permissions.sh`. They are named in numerical order and all of these scripts are always run.

## Crate Structure
This Cargo workspace has two crates under `crates/`, each with its own dependencies. There is no shared lib crate; the `bones.toml` structs are duplicated since each binary discovers and uses config differently.

```
gitbones/
├── Cargo.toml                  # workspace root
├── kit/                        # embedded assets (scaffolding templates)
│   ├── bones.toml
│   ├── deployment/
│   └── hooks/
├── crates/
│   ├── gitbones/               # local CLI binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs         # clap setup, command dispatch
│   │       ├── commands/
│   │       │   ├── mod.rs
│   │       │   ├── init.rs
│   │       │   ├── doctor.rs
│   │       │   ├── push.rs
│   │       │   └── version.rs
│   │       ├── config.rs       # bones.toml structs + load/save + local file discovery
│   │       ├── embedded.rs     # rust-embed from kit/, scaffold writing
│   │       ├── git.rs          # git2 operations: read remote URLs, repo validation
│   │       ├── prompts.rs      # interactive user input collection, returns config
│   │       └── ssh.rs          # openssh session management + rsync
│   └── gitbones-remote/        # server-side binary
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── commands/
│           │   ├── mod.rs
│           │   ├── init.rs
│           │   ├── doctor.rs
│           │   ├── pre_deploy.rs
│           │   ├── post_deploy.rs
│           │   └── version.rs
│           ├── config.rs       # bones.toml structs + remote file discovery
│           └── permissions.rs  # chown/chmod logic
└── docs/
```

### Gitbones CLI Commands
- **init**:
  - Informs the user that there should be a remote git url set up, explains what's going to happen, and then asks the user for permission to proceed.
  - Gets or creates the `.bones` folder with our default scaffolding.
  - Updates `.gitignore` to add .bones folder.
  - Loads existing config from `.bones/bones.toml` or collects new user input via prompts.
  - Creates upstream bare repository on remote using the url set in `git remote [remote_name]`, setting it up if it doesn't exist. We fail if no git remote URL is set.
  - Builds and uploads post-receive hook to remote.
  - Saves config to `.bones/bones.toml`.

- **doctor**
  - This command checks all concerns in your local environment.
  - Loads config from `.bones/bones.toml`
  - Runs local checks:
    - `.bones` folder is set up correctly. Deployment scripts are named appropriately.
    - Local `pre-push` hook is symlinked properly.
  - Runs remote checks (skipped with `--local`):
    - `gitbones-remote` executable exists on remote and can be run globally.
    - `{project_name}.git/bones` folder exists on remote (needs `gitbones push` warning)
    - `{project_name}.git/bones/hooks` matches with `{project_name}.git/hooks` inside the remote bare repo.
  - The `--local` flag skips all remote checks. The `pre-push` hook uses this flag since the remote is independently validated by `gitbones-remote doctor` in the `pre-receive` hook.

- **push**
  - Uses an `rsync -av --delete` command to push up our local `.bones` folder to the bare repo.
  - We will create a `bones` folder under our `{project_name}.git/` folder so that everything is self-contained inside git.
  - Deletes sample git hooks in bare repo, so that our files will be the only files to worry about in the `{project_name}.git/hooks` folder.
  - Runs commands on remote that symlinks our `{project_name}.git/bones/hooks` files are symlinked with `{project_name}.git/hooks` properly.

- **version**:
  - Echoes "gitbones 0.1.0".

### GitbonesRemote CLI Commands
- **init**:
  - Must be run as sudo.
  - Installs a drop-in file at `/etc/sudoers.d/gitbones` to allow `gitbones-remote` commands without requiring password.
- **doctor**:
  - Checks to see if the server is set up properly:
    - `gitbones-remote` can be run without requiring password
    - `gitbones-remote` is globally available.
- **pre-deploy**
	- Sets all file ownership to git for the deployment. 
- **post-deploy**
	- Runs a permissions hardening function setting all permissions back to the layout configured in `bones.toml`, like for instance setting everything back to be owned by the service user. 
- **version**:
  - Echoes "gitbones 0.1.0".

## Security Notes
- Sudo access for the deployment user is strictly limited to passwordless execution of gitbones-remote commands via /etc/sudoers configuration.
- No broader sudo privileges are granted.
- All operations are audited through system logs.

## Flow
- User runs `gitbones init`, and the procedures outlined above are executed.
- User can make any changes to their deployment scripts or hooks in `.bones/` (e.g., customizing `deployment/` files or adding project-specific logic).
- User runs `gitbones push` to sync the `.bones/` folder to the remote bare repo.
- User runs `git push production master` or some similar command where the remote name aligns with our bones.toml configuration.
- The `pre-push` hook checks to see if we are pushing to our bones remote (in this example, `production`). If so, it runs `gitbones doctor` and fails on warnings/errors.
- On remote: `pre-receive` runs `gitbones-remote doctor` (fails on issues), then calls `pre-deploy` (which runs `gitbones-remote pre-deploy` for ownership changes).
- `post-receive` updates the worktree to the latest commit, then calls `deploy` (runs all `deployment/` scripts sequentially).
- Finally, `post-deploy` calls `gitbones-remote post-deploy` to harden permissions based on bones.toml.

## Cargo Dependencies
- clap
- git2
- inquire
- rust-embed
- toml
- rsync
- openssh
- serde (derive)
- tokio
- console
- nix
- walkdir
- anyhow