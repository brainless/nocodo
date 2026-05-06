use std::path::Path;

const TEMPLATE_REPO_URL: &str = "https://github.com/brainless/rustysolid.git";

/// Clone template repo into project_path.
/// Skips silently if directory already has files.
pub async fn clone_template_repo(project_path: String) {
    let path = Path::new(&project_path);

    // Skip if directory already has files
    if path.exists() {
        if let Ok(mut entries) = path.read_dir() {
            if entries.next().is_some() {
                log::info!(
                    "Project directory {} already has files, skipping template clone",
                    project_path
                );
                return;
            }
        }
    }

    log::info!("Cloning template repo to {}", project_path);

    match tokio::task::spawn_blocking(move || {
        async_std::task::block_on(clone_async(&project_path))
    })
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(e)) => log::warn!("Template clone failed: {}", e),
        Err(e) => log::warn!("Template clone task panicked: {}", e),
    }
}

async fn clone_async(project_path: &str) -> Result<(), String> {
    let path = Path::new(project_path);

    let url = gix::url::parse(TEMPLATE_REPO_URL.into())
        .map_err(|e| format!("Failed to parse URL: {}", e))?;

    let mut prepare_clone = gix::prepare_clone(url, path)
        .map_err(|e| format!("Failed to prepare clone: {}", e))?;

    let (repo, _outcome) = prepare_clone
        .fetch_only(gix::progress::Discard, &gix::interrupt::IS_INTERRUPTED)
        .await
        .map_err(|e| format!("Failed to fetch: {}", e))?;

    // Checkout HEAD into working tree
    let workdir = repo.workdir().ok_or("Repository has no workdir")?;
    let head_commit = repo.head_commit().map_err(|e| format!("No HEAD: {}", e))?;
    let tree = head_commit.tree().map_err(|e| format!("No tree: {}", e))?;

    let mut index = repo
        .index_from_tree(&tree.id)
        .map_err(|e| format!("Index from tree: {}", e))?;

    let mut opts = repo
        .checkout_options(gix::worktree::stack::state::attributes::Source::IdMapping)
        .map_err(|e| format!("Checkout options: {}", e))?;
    opts.destination_is_initially_empty = true;

    gix::worktree::state::checkout(
        &mut index,
        workdir,
        repo.objects.clone().into_arc().map_err(|e| format!("Objects arc: {}", e))?,
        &gix::progress::Discard,
        &gix::progress::Discard,
        &gix::interrupt::IS_INTERRUPTED,
        opts,
    )
    .map_err(|e| format!("Checkout failed: {}", e))?;

    let _ = index.write(Default::default());

    log::info!("Template cloned successfully to {}", project_path);
    Ok(())
}
