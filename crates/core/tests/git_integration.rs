use std::path::Path;
use std::process::Command;

use tempfile::TempDir;

use anvl_core::workspace::git::{
    commit, diff_file, refresh_git, stage_file, unstage_file,
};

// ---------------------------------------------------------------------------
// Helper: initialise a throwaway git repo inside a TempDir
// ---------------------------------------------------------------------------

fn git_init(dir: &Path) {
    Command::new("git")
        .args(["init"])
        .current_dir(dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.email", "test@test.com"])
        .current_dir(dir)
        .output()
        .unwrap();
    Command::new("git")
        .args(["config", "user.name", "Test"])
        .current_dir(dir)
        .output()
        .unwrap();
}

fn write_file(dir: &Path, name: &str, content: &str) {
    std::fs::write(dir.join(name), content).unwrap();
}

fn git_add_all(dir: &Path) {
    Command::new("git")
        .args(["add", "-A"])
        .current_dir(dir)
        .output()
        .unwrap();
}

fn git_commit(dir: &Path, message: &str) {
    Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(dir)
        .output()
        .unwrap();
}

fn git_branch(dir: &Path, branch: &str) {
    Command::new("git")
        .args(["branch", branch])
        .current_dir(dir)
        .output()
        .unwrap();
}

// ===========================================================================
// C1 — refresh_git with a real temp repo
// ===========================================================================

#[tokio::test]
async fn refresh_git_clean_repo() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    git_init(dir);
    write_file(dir, "hello.txt", "hello");
    git_add_all(dir);
    git_commit(dir, "initial commit");

    let state = refresh_git(dir, None).await.unwrap();

    // Branch should be "main" or "master" depending on git config.
    let branch = state.branch.as_deref().unwrap();
    assert!(
        branch == "main" || branch == "master",
        "expected main or master, got {branch}"
    );

    // No uncommitted changes.
    assert!(state.changed.is_empty(), "expected no changed files");
}

#[tokio::test]
async fn refresh_git_dirty_worktree() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    git_init(dir);
    write_file(dir, "hello.txt", "hello");
    git_add_all(dir);
    git_commit(dir, "initial commit");

    // Modify a tracked file without staging.
    write_file(dir, "hello.txt", "hello world");

    let state = refresh_git(dir, None).await.unwrap();
    assert_eq!(state.changed.len(), 1, "expected 1 changed file");
    assert_eq!(state.changed[0].path, "hello.txt");
}

#[tokio::test]
async fn refresh_git_shows_new_branch() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    git_init(dir);
    write_file(dir, "hello.txt", "hello");
    git_add_all(dir);
    git_commit(dir, "initial commit");

    git_branch(dir, "feature-x");

    let state = refresh_git(dir, None).await.unwrap();
    let branch_names: Vec<&str> = state
        .local_branches
        .iter()
        .map(|b| b.name.as_str())
        .collect();
    assert!(
        branch_names.contains(&"feature-x"),
        "expected feature-x in {branch_names:?}"
    );
}

// ===========================================================================
// C2 — stage_file / unstage_file / commit
// ===========================================================================

#[tokio::test]
async fn stage_and_unstage_file() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    git_init(dir);
    write_file(dir, "a.txt", "aaa");
    git_add_all(dir);
    git_commit(dir, "initial commit");

    // Modify the file.
    write_file(dir, "a.txt", "aaa modified");

    // Stage it.
    stage_file(dir, "a.txt", None).await.unwrap();
    let state = refresh_git(dir, None).await.unwrap();
    assert_eq!(state.changed.len(), 1);
    // After staging, the index_status should reflect the change (not ' ' and not '?').
    let f = &state.changed[0];
    assert_eq!(f.path, "a.txt");
    assert!(
        f.index_status != ' ' && f.index_status != '?',
        "expected staged index_status, got '{}'",
        f.index_status
    );

    // Unstage it.
    unstage_file(dir, "a.txt", None).await.unwrap();
    let state = refresh_git(dir, None).await.unwrap();
    assert_eq!(state.changed.len(), 1);
    let f = &state.changed[0];
    assert_eq!(f.path, "a.txt");
    // After unstaging the index_status should be clean (' ') and the worktree dirty.
    assert_eq!(f.index_status, ' ', "expected ' ' index_status after unstage");
    assert_eq!(f.worktree_status, 'M', "expected 'M' worktree_status after unstage");
}

#[tokio::test]
async fn stage_and_commit_clears_changes() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    git_init(dir);
    write_file(dir, "b.txt", "bbb");
    git_add_all(dir);
    git_commit(dir, "initial commit");

    // Modify, stage, and commit via the library functions.
    write_file(dir, "b.txt", "bbb modified");
    stage_file(dir, "b.txt", None).await.unwrap();
    commit(dir, "second commit", None).await.unwrap();

    let state = refresh_git(dir, None).await.unwrap();
    assert!(state.changed.is_empty(), "expected no changed files after commit");
    assert!(
        state.recent_commits.iter().any(|c| c.message == "second commit"),
        "expected 'second commit' in recent_commits: {:?}",
        state.recent_commits
    );
}

// ===========================================================================
// C3 — diff_file for tracked, untracked, and clean files
// ===========================================================================

#[tokio::test]
async fn diff_file_modified_tracked() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    git_init(dir);
    write_file(dir, "c.txt", "original");
    git_add_all(dir);
    git_commit(dir, "initial commit");

    write_file(dir, "c.txt", "modified");

    let diff = diff_file(dir, "c.txt", None).await.unwrap();
    assert!(
        !diff.trim().is_empty(),
        "expected non-empty diff for modified tracked file"
    );
    assert!(
        diff.contains('+') || diff.contains('-'),
        "expected diff hunks"
    );
}

#[tokio::test]
async fn diff_file_untracked_new() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    git_init(dir);
    write_file(dir, "tracked.txt", "tracked");
    git_add_all(dir);
    git_commit(dir, "initial commit");

    // Create an untracked file.
    write_file(dir, "new.txt", "brand new content");

    let diff = diff_file(dir, "new.txt", None).await.unwrap();
    assert!(
        !diff.trim().is_empty(),
        "expected non-empty synthetic diff for untracked file"
    );
    assert!(
        diff.contains("+brand new content"),
        "expected '+' prefixed lines in synthetic diff, got:\n{diff}"
    );
}

#[tokio::test]
async fn diff_file_clean_committed() {
    let tmp = TempDir::new().unwrap();
    let dir = tmp.path();

    git_init(dir);
    write_file(dir, "d.txt", "committed");
    git_add_all(dir);
    git_commit(dir, "initial commit");

    // File is committed and unmodified — diff should be empty.
    let diff = diff_file(dir, "d.txt", None).await.unwrap();
    assert!(
        diff.trim().is_empty(),
        "expected empty diff for clean committed file, got:\n{diff}"
    );
}
