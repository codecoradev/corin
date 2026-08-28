<!--
PR title must follow Conventional Commits — it becomes the squash commit message.
Format: type(scope): short description
Examples: fix(recall): handle empty query / feat(server): add health endpoint / docs(readme): update install instructions
-->

## What
<!-- One or two sentences describing the change. -->

## Why
<!-- The problem you're solving. Link to the issue if there is one (e.g. "Closes #42"). -->

## How
<!-- Brief notes on the approach, only if non-obvious. -->

## Testing

- [ ] `cargo fmt --all -- --check` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo test` passes
- [ ] `npx svelte-check --tsconfig ./tsconfig.json` passes (0 errors)
- [ ] `npm run test -- --run` passes
- [ ] `npm run build` succeeds
- [ ] [Cora review](https://github.com/codecoradev/cora-cli) run locally (`cora review --base origin/develop`)
- [ ] Manual smoke-test of the affected feature <!-- describe what you tested -->

<!-- If you touched a load-bearing subsystem (web transport in web-routes.ts, lifecycle endpoints, graph handling, import/export), add specific test details here. -->
