# 🧩 Contributing Guidelines

Thank you for contributing!  
Please review the following conventions before opening a Pull Request or committing changes.

---

## 🪜 1. Branch Naming Convention

All branches must follow the structure:

```

<type>/<scope>(/<sub-scope>)/<task>

```

### ✅ Examples
```

feature/game/ecs-refactor
feature/runtime/scheduler/add-priority-queue
fix/zk/proof/poseidon-constraint
refactor/client/ui/overlay-system

```

### ✅ Types
| Type | Description |
|------|--------------|
| `feature` | New feature development |
| `fix` | Bug fixes |
| `refactor` | Code refactoring or structural improvements |
| `chore` | Build, config, or maintenance changes |
| `docs` | Documentation updates |
| `test` | Test-related updates |
| `research` | Experimental or prototype work |

### ✅ Scopes
The valid top-level scopes are:

- `game` — Game logic, ECS systems, entity management  
- `runtime` — Execution engine, scheduler, resource management  
- `zk` — Zero-knowledge circuits, proofs, and verifiers  
- `client` — UI, networking, or frontend components  

> The optional **sub-scope** can be added when the module or subsystem is clearly defined.

---

## 🧾 2. Commit Message Convention

We follow the [Conventional Commits](https://www.conventionalcommits.org/) standard.

### Format
```

<type>(<scope>): <short summary>

```

> The `scope` should generally be included for clarity, but may be omitted if it’s not applicable.


### ✅ Examples
```

feat(game): add turn-based ECS system
fix(zk): resolve constraint overflow
refactor(runtime/scheduler): simplify priority handling
docs(client): update README build instructions

```

### ✅ Allowed Types
- `feat`: new feature  
- `fix`: bug fix  
- `refactor`: code refactoring without changing behavior  
- `docs`: documentation only  
- `test`: adding or modifying tests  
- `chore`: build, dependency, or configuration changes  
- `style`: formatting or stylistic updates  

### ✅ Rules
- Use **imperative mood** (`add`, `fix`, `remove`, `update`, etc.)
- Keep subject lines **under 72 characters**
- Separate subject and body with a blank line (if body exists)
- For breaking changes, include:
```

BREAKING CHANGE: zk proof API interface changed

````

---

## 🌿 3. Pull Request Guidelines

- Each PR should focus on **a single feature, fix, or refactor**  
→ Small, atomic PRs are easier to review and merge.
- The PR title should follow the same format as commit messages.  
Example: `feat(runtime): add parallel scheduler`
- Clearly describe the main changes in the PR body.
- All CI checks must pass before merging.
- Use `@mention` to request specific reviewers if needed.
