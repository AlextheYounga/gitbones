use anyhow::{Context, Result, bail};
use openssh::{KnownHosts, Session, SessionBuilder, Stdio};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::config::BonesConfig;

pub async fn connect(config: &BonesConfig) -> Result<Session> {
    let host = &config.data.host;
    let port: u16 = config
        .data
        .port
        .parse()
        .with_context(|| format!("Invalid port: {}", config.data.port))?;
    let user = &config.permissions.defaults.deploy;

    let session = SessionBuilder::default()
        .known_hosts_check(KnownHosts::Accept)
        .user(user.clone())
        .port(port)
        .connect(host)
        .await
        .with_context(|| format!("Failed to connect to {user}@{host}:{port}"))?;

    Ok(session)
}

pub async fn run_cmd(session: &Session, cmd: &str) -> Result<String> {
    let output = session
        .command("bash")
        .arg("-c")
        .arg(cmd)
        .output()
        .await
        .with_context(|| format!("Failed to execute remote command: {cmd}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("Remote command failed: {cmd}\n{stderr}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub async fn stream_cmd(session: &Session, cmd: &str) -> Result<()> {
    let mut child = session
        .command("bash")
        .arg("-c")
        .arg(cmd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .await
        .with_context(|| format!("Failed to execute remote command: {cmd}"))?;

    let stdout = child.stdout().take().expect("stdout was piped");
    let stderr = child.stderr().take().expect("stderr was piped");

    let stdout_task = tokio::spawn(async move {
        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            println!("{line}");
        }
    });

    let stderr_task = tokio::spawn(async move {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        while let Ok(Some(line)) = lines.next_line().await {
            eprintln!("{line}");
        }
    });

    // Drain both streams concurrently before checking exit status
    let _ = tokio::join!(stdout_task, stderr_task);

    let status = child
        .wait()
        .await
        .context("Failed to wait for remote command")?;

    if !status.success() {
        bail!("Remote command failed: {cmd}");
    }

    Ok(())
}

pub async fn create_bare_repo(session: &Session, git_dir: &str) -> Result<()> {
    let check = format!("test -d {git_dir}");
    if session
        .command("bash")
        .arg("-c")
        .arg(&check)
        .status()
        .await?
        .success()
    {
        println!("Bare repo already exists at {git_dir}");
        return Ok(());
    }

    println!("Creating bare repo at {git_dir}...");
    run_cmd(session, &format!("git init --bare {git_dir}")).await?;
    Ok(())
}

pub async fn upload_post_receive(
    session: &Session,
    git_dir: &str,
    hook_content: &str,
) -> Result<()> {
    let hook_path = format!("{git_dir}/hooks/post-receive");

    // Write hook content via heredoc
    let cmd = format!(
        "cat > '{hook_path}' << 'GITBONES_EOF'\n{hook_content}\nGITBONES_EOF\nchmod +x '{hook_path}'"
    );
    run_cmd(session, &cmd).await?;
    println!("Uploaded post-receive hook to {hook_path}");
    Ok(())
}
