# GitHub Follow Sync

Small Rust utility to follow GitHub followers and unfollow accounts that stop following you. Runs in GitHub Actions every three hours, no server needed.

## Setup

1. Create a GitHub repository and upload all the files, including `.github/workflows/follower-sync.yml`, to its default branch.
2. Create a [fine-grained personal access token](https://github.com/settings/personal-access-tokens/new) **for your personal account**, with **Account permissions → Followers → Read and write**. No repository permissions are needed for this token.
3. Go to your repository's **Settings → Secrets and variables → Actions → New repository secret**. Name the secret `GH_FOLLOW_TOKEN` and paste your token.
4. Ensure the repository's **Settings → Actions → General → Workflow permissions** allows `Read and write permissions`, and that the default branch allows Actions to push commits.
5. Open **Actions → Follow-back sync → Run workflow** to start it immediately, or let the schedule trigger automatically.

## Behaviour

- Checks the authenticated account's followers and following lists every three hours, at minute 17 UTC. GitHub may delay scheduled runs.
- Follows accounts that follow you but you do not follow.
- Unfollows only accounts previously followed **by this utility** that have stopped following you.
- Existing manual subscriptions remain untouched. Account renames are handled using GitHub numeric IDs.
- Performs at most 10 follow/unfollow operations per run; remaining work waits for subsequent runs.
- Saves `state.json` after each successful operation. The workflow commits changes even if a later operation fails.
- Stops on incomplete API lists rather than acting on incomplete data (limit: 10,000 accounts per list).

The repository's `GITHUB_TOKEN` is used only to save `state.json`. The `GH_FOLLOW_TOKEN` secret is used only for personal follow operations. Never commit the personal token.

A branch protection rule preventing GitHub Actions from pushing will also prevent state persistence. If the follow operation succeeds but the state commit fails, state may become stale. Review failed workflow runs. Automated follow activity may also be subject to GitHub abuse protections; the rate cap is not a guarantee of policy compliance.

## Local run

Set `GH_FOLLOW_TOKEN` in your environment, then run `cargo run --release`. Local runs update `state.json`, but you must commit that file manually if you also use GitHub Actions.
