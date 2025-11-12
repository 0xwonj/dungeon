# Sui Move Best Practices

> Essential guidelines for writing clean, secure, and maintainable Sui Move smart contracts.

## Table of Contents
- [Naming Conventions](#naming-conventions)
- [Module Organization](#module-organization)
- [Documentation](#documentation)
- [Struct Design](#struct-design)
- [Function Patterns](#function-patterns)
- [Security](#security)
- [Testing](#testing)

---

## Naming Conventions

### Constants
```move
// ✅ Regular constants: SCREAMING_SNAKE_CASE
const MAX_PLAYERS: u64 = 100;
const DEFAULT_TIMEOUT: u64 = 3600;

// ✅ Error codes: PascalCase with E prefix
const ENotOwner: u64 = 1;
const EAlreadyFinalized: u64 = 2;
const EInsufficientBalance: u64 = 3;

// ❌ Wrong: Don't use E_SCREAMING_SNAKE_CASE
const E_NOT_OWNER: u64 = 1;  // Wrong!
```

### Structs
```move
// ✅ Event structs: Use Event suffix
public struct SessionStartedEvent has copy, drop { ... }
public struct TransferCompletedEvent has copy, drop { ... }

// ✅ Abilities in order: key, copy, drop, store
public struct GameSession has key, store { ... }
public struct Config has copy, drop { ... }

// ❌ Wrong: Don't use 'potato' in names
public struct SessionPotato has key { ... }  // Wrong!

// ❌ Wrong: Wrong ability order
public struct GameSession has store, key { ... }  // Wrong!
```

### Functions
```move
// ✅ Standard CRUD naming patterns
public fun new(): Object { ... }                    // Empty object
public fun create(params): Object { ... }           // Initialize with data
public fun add_item(list, item) { ... }             // Add to collection
public fun remove_item(list, id) { ... }            // Remove from collection
public fun borrow_state(obj): &State { ... }        // Immutable reference
public fun borrow_mut_state(obj): &mut State { ... } // Mutable reference
public fun destroy(obj) { ... }                     // Destroy object

// ✅ Descriptive names for complex operations
public fun update(session: &mut GameSession, ...) { ... }
public fun finalize(session: &mut GameSession) { ... }
```

---

## Module Organization

```move
/// Module documentation
///
/// High-level description of module purpose and design.
module package::module_name {
    // Import only what's needed (many Sui modules are auto-imported)
    use sui::event;

    // ===== Error Codes =====

    const ENotOwner: u64 = 1;
    const EInvalidState: u64 = 2;

    // ===== Constants =====

    const MAX_SUPPLY: u64 = 10000;
    const MIN_DEPOSIT: u64 = 1000;

    // ===== Structs =====

    /// Main data structures
    public struct MainObject has key { ... }

    // ===== Events =====

    /// Event definitions
    public struct CreatedEvent has copy, drop { ... }

    // ===== Public Functions =====

    /// User-facing API functions
    public fun create(...) { ... }
    public fun update(...) { ... }

    // ===== Package Functions =====

    /// Internal package functions (cross-module within package)
    public(package) fun internal_helper(...) { ... }

    // ===== Private Functions =====

    /// Module-only helper functions
    fun validate(...) { ... }
    fun compute_hash(...) { ... }

    // ===== View Functions =====

    /// Read-only accessors
    public fun owner(obj: &Object): address { ... }
    public fun borrow_data(obj: &Object): &Data { ... }
}
```

**Key Principles:**
- Use `===== Section Name =====` comments for clear separation
- Group functions by visibility and purpose
- Keep related code together

---

## Documentation

### Function Documentation
```move
/// Create a new game session
///
/// Initializes a new GameSession object with the provided commitments.
/// The session is returned to the caller for ownership transfer.
///
/// # Arguments
/// * `oracle_root` - Content hash of oracle data (maps, items, NPCs)
/// * `initial_state_root` - Merkle root of initial game state
/// * `seed_commitment` - Commitment to RNG seed (hash of seed)
/// * `ctx` - Transaction context
///
/// # Returns
/// A new GameSession object owned by the caller
///
/// # Aborts
/// * `EInvalidRoot` - If any root is empty or invalid
///
/// # Events
/// Emits `SessionStartedEvent` with session details
///
/// # Example
/// ```
/// let session = game_session::create(
///     b"oracle_hash",
///     b"state_hash",
///     b"seed_hash",
///     ctx
/// );
/// transfer::transfer(session, player_address);
/// ```
public fun create(
    oracle_root: vector<u8>,
    initial_state_root: vector<u8>,
    seed_commitment: vector<u8>,
    ctx: &mut TxContext,
): GameSession {
    // Implementation
}
```

**Documentation Levels:**
- `///` - Public API documentation (appears in generated docs)
- `//` - Implementation notes (for developers reading code)

### Struct Documentation
```move
/// Represents an active game session
///
/// A GameSession tracks all commitments needed for verifiable gameplay:
/// - Oracle commitment (content-addressed game rules)
/// - State commitment (current game state merkle root)
/// - Action commitment (accumulated player actions)
public struct GameSession has key, store {
    id: UID,
    /// Player address (session owner)
    player: address,

    // Immutable context (set at creation)
    /// Oracle data commitment (content hash)
    oracle_root: vector<u8>,
    /// Initial state root at game start
    initial_state_root: vector<u8>,

    // Mutable state (updated by proofs)
    /// Current game state root
    state_root: vector<u8>,
}
```

---

## Struct Design

### Ability Declaration
```move
// Object types (owned or shared)
public struct GameSession has key, store { id: UID, ... }

// Value types (copyable)
public struct Config has copy, drop, store { ... }

// Events (must be copy + drop)
public struct CreatedEvent has copy, drop { ... }

// Hot Potato (no abilities - must be consumed)
public struct Receipt { amount: u64 }
```

**Ability Order:** Always declare in order: `key, copy, drop, store`

### Field Organization
```move
public struct GameSession has key, store {
    id: UID,

    // Group related fields with comments
    // Ownership
    player: address,

    // Immutable context
    oracle_root: vector<u8>,
    seed_commitment: vector<u8>,
    started_at: u64,

    // Mutable state
    state_root: vector<u8>,
    turn_count: u32,
    last_update: u64,

    // Status flags
    finalized: bool,
}
```

---

## Function Patterns

### Ownership and Transfer
```move
// Return object for caller to transfer
public fun create(..., ctx: &mut TxContext): GameSession {
    let session = GameSession { ... };
    session  // Caller decides where to send it
}

// Transfer to specific address
public fun create_and_transfer(recipient: address, ctx: &mut TxContext) {
    let session = GameSession { ... };
    transfer::transfer(session, recipient);
}

// Share object (multiple readers/writers)
public fun create_shared(ctx: &mut TxContext) {
    let registry = Registry { ... };
    transfer::share_object(registry);
}

// Freeze object (immutable forever)
public fun create_immutable(ctx: &mut TxContext) {
    let config = Config { ... };
    transfer::freeze_object(config);
}
```

### Reference Patterns
```move
// Borrow immutable reference (no copy overhead)
public fun borrow_state(session: &GameSession): &vector<u8> {
    &session.state_root
}

// Borrow mutable reference
public fun borrow_mut_state(session: &mut GameSession): &mut vector<u8> {
    &mut session.state_root
}

// Return primitive values (cheap to copy)
public fun turn_count(session: &GameSession): u32 {
    session.turn_count
}
```

### Capability Pattern
```move
/// Admin capability for privileged operations
public struct AdminCap has key, store {
    id: UID,
}

/// Only admin can call this
public fun admin_operation(
    _cap: &AdminCap,  // Proof of admin rights
    target: &mut SomeObject,
) {
    // Privileged operation
}

/// Create and transfer admin capability (called once in init)
fun init(ctx: &mut TxContext) {
    let admin_cap = AdminCap { id: object::new(ctx) };
    transfer::transfer(admin_cap, tx_context::sender(ctx));
}
```

### Hot Potato Pattern
```move
/// Must be consumed (no abilities)
public struct Receipt {
    session_id: address,
    amount: u64,
}

/// Create receipt that MUST be consumed
public fun create_receipt(session_id: address, amount: u64): Receipt {
    Receipt { session_id, amount }
}

/// Only way to get rid of receipt
public fun consume_receipt(receipt: Receipt, target: &mut Account) {
    let Receipt { session_id: _, amount } = receipt;
    target.balance = target.balance + amount;
    // Receipt is destroyed by destructuring
}
```

---

## Security

### Always Validate Ownership
```move
public fun update(session: &mut GameSession, ctx: &TxContext) {
    // ✅ Always check caller is owner
    assert!(tx_context::sender(ctx) == session.player, ENotOwner);

    // ... rest of logic
}
```

### Validate State Preconditions
```move
public fun update(session: &mut GameSession, ctx: &TxContext) {
    assert!(tx_context::sender(ctx) == session.player, ENotOwner);

    // ✅ Check state is valid for operation
    assert!(!session.finalized, EAlreadyFinalized);
    assert!(session.balance >= amount, EInsufficientBalance);

    // ... perform update
}
```

### Emit Events for Important Actions
```move
public fun update(session: &mut GameSession, new_state: vector<u8>) {
    // ... validation and updates

    // ✅ Emit event for observability
    event::emit(SessionUpdatedEvent {
        session_id: object::uid_to_address(&session.id),
        new_state_root: new_state,
        updated_at: tx_context::epoch(ctx),
    });
}
```

### Use Type Safety
```move
// ✅ Good: Type ensures validity
public fun update(session: &mut GameSession, proof: Proof) {
    // session and proof are guaranteed to be valid types
}

// ❌ Avoid: Raw addresses/IDs without validation
public fun update_by_id(session_id: address, data: vector<u8>) {
    // Hard to validate, error-prone
}
```

### Checked Arithmetic
```move
// ✅ Move has overflow checks by default
let total = base + bonus;  // Aborts on overflow

// For explicit handling
use sui::math;
let result = base.wrapping_add(bonus);  // Wraps instead of aborting
```

---

## Testing

### Test Structure
```move
#[test_only]
module package::module_tests {
    use sui::test_scenario;
    use package::module_name;

    #[test]
    fun test_create_and_update() {
        let mut scenario = test_scenario::begin(@0xA);

        // Setup
        {
            let ctx = test_scenario::ctx(&mut scenario);
            let session = module_name::create(b"oracle", b"state", b"seed", ctx);
            transfer::public_transfer(session, @0xA);
        };

        // Test operation
        test_scenario::next_tx(&mut scenario, @0xA);
        {
            let mut session = test_scenario::take_from_sender<GameSession>(&scenario);
            let ctx = test_scenario::ctx(&mut scenario);

            module_name::update(&mut session, b"new_state", b"actions", 10, ctx);

            assert!(module_name::turn_count(&session) == 10, 0);

            test_scenario::return_to_sender(&scenario, session);
        };

        test_scenario::end(scenario);
    }

    #[test]
    #[expected_failure(abort_code = module_name::ENotOwner)]
    fun test_unauthorized_update() {
        let mut scenario = test_scenario::begin(@0xA);

        // Create as 0xA
        {
            let ctx = test_scenario::ctx(&mut scenario);
            let session = module_name::create(b"oracle", b"state", b"seed", ctx);
            transfer::public_transfer(session, @0xA);
        };

        // Try to update as 0xB (should fail)
        test_scenario::next_tx(&mut scenario, @0xB);
        {
            let mut session = test_scenario::take_from_address<GameSession>(&scenario, @0xA);
            let ctx = test_scenario::ctx(&mut scenario);

            module_name::update(&mut session, b"new_state", b"actions", 10, ctx);

            test_scenario::return_to_address(@0xA, session);
        };

        test_scenario::end(scenario);
    }
}
```

### Test Coverage
```bash
# Run tests with coverage
sui move test --coverage

# Aim for high coverage on critical paths
# - Happy path scenarios
# - Error conditions (unauthorized access, invalid state)
# - Edge cases (empty vectors, zero values, max values)
```

---

## Quick Reference

### Move.toml Dependencies

**Sui 1.45+ (2024)**: Framework dependencies are auto-added

```toml
# ✅ Good: Let Sui auto-add framework dependencies
[package]
name = "my_package"
edition = "2024.beta"

[dependencies]
# Empty or only third-party packages

[addresses]
my_package = "0x0"
```

```toml
# ❌ Avoid: Explicit Sui dependency (causes warning)
[dependencies]
Sui = { git = "https://github.com/MystenLabs/sui.git", ... }
# This disables auto-dependency feature
```

```toml
# ✅ Exception: Override when you need specific version
[dependencies]
Sui = { git = "...", rev = "specific-commit", override = true }
# Use 'override = true' to explicitly control version
```

**Auto-added packages** (no need to declare):
- `Sui` - Sui framework
- `MoveStdlib` - Move standard library
- `SuiSystem` - Sui system packages
- `Bridge` - Bridge framework
- `Deepbook` - Deepbook DEX

### Import Guidelines
```move
// Most Sui modules are auto-imported, only import when needed:
use sui::event;           // ✅ Need to import
use sui::coin;            // ✅ Need to import
use sui::balance;         // ✅ Need to import

// These are auto-imported (don't need explicit import):
// - sui::object
// - sui::tx_context
// - sui::transfer
```

### Common Patterns Summary

| Pattern | Use Case | Example |
|---------|----------|---------|
| **Owned Object** | Single owner, transferable | `GameSession has key, store` |
| **Shared Object** | Multiple readers/writers | `transfer::share_object(registry)` |
| **Frozen Object** | Immutable forever | `transfer::freeze_object(config)` |
| **Capability** | Access control | `AdminCap has key, store` |
| **Hot Potato** | Must be consumed | `Receipt` (no abilities) |
| **Event** | Observability | `Event has copy, drop` |

### Error Code Ranges (Convention)
```move
// 1-99: Ownership/Authorization
const ENotOwner: u64 = 1;
const ENotAuthorized: u64 = 2;

// 100-199: State validation
const EAlreadyFinalized: u64 = 100;
const EInvalidState: u64 = 101;

// 200-299: Value validation
const EInsufficientBalance: u64 = 200;
const EAmountTooLarge: u64 = 201;

// 300+: Domain-specific errors
const EInvalidProof: u64 = 300;
const EOracleNotFound: u64 = 301;
```

---

## Resources

- **Official Docs**: https://docs.sui.io/concepts/sui-move-concepts/conventions
- **Move Book**: https://move-book.com/
- **Sui Examples**: https://github.com/MystenLabs/sui/tree/main/examples
- **Style Guide**: https://docs.sui.io/style-guide

---

## Checklist for Code Review

Before submitting your Move code, verify:

- [ ] Error constants use `EName` format (not `E_NAME`)
- [ ] Event structs have `Event` suffix
- [ ] Struct abilities in order: `key, copy, drop, store`
- [ ] Functions follow CRUD naming (`create`, `add`, `remove`, `borrow`, etc.)
- [ ] All public functions have doc comments
- [ ] Ownership validation in all mutations
- [ ] State preconditions checked with `assert!`
- [ ] Events emitted for important state changes
- [ ] Tests cover happy path and error cases
- [ ] No unnecessary imports (most Sui modules auto-imported)
- [ ] View functions return references when possible
- [ ] Module organized with clear section comments

---

**Move Edition**: 2024.beta
