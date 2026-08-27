# Design principles

> Load when: implementing or reviewing new behavior, refactors, public APIs, or architecture.
> Skip when: the change is documentation-only, generated output, or mechanical formatting.

## Core rules

- **Correctness first:** define inputs, outputs, invariants, failure behavior, and ownership before
  choosing abstractions.
- **KISS:** use the smallest design that clearly satisfies current requirements. Prefer direct
  control flow and domain names over framework-like indirection.
- **YAGNI:** do not add extension points, configuration, generic parameters, or layers for an
  unrequested future use case.
- **DRY with evidence:** remove duplicated knowledge, not merely similar syntax. Wait for a stable
  shared concept, commonly after three real occurrences, before extracting it.
- **High cohesion, low coupling:** keep behavior with the state and invariant it owns. Avoid modules
  that coordinate unrelated concerns or reach through several layers.
- **Composition over inheritance-like hierarchies:** combine focused types and traits. Avoid broad
  traits that force unrelated implementations to depend on unused behavior.
- **Explicit dependencies:** pass required collaborators or configuration at construction or call
  sites. Avoid hidden global state and action at a distance.
- **Fail explicitly:** return actionable errors at the boundary that can handle them. Never present
  a stub, fallback, or partial result as successful work.
- **Leave code no worse:** fix small nearby defects only when the correction is safe and inseparable
  from the task. Record broader cleanup as separate work.

## SOLID adapted to Rust

### Single responsibility

A module or type should own one cohesive set of invariants and have one primary reason to change.
Split orchestration from computation and policy from I/O when they evolve or test differently. Do
not split a readable function solely because it has many lines.

### Open/closed

Prefer data and behavior that can accept a new supported case without editing unrelated modules.
Use an enum when the set of cases is closed and exhaustive matching is valuable. Use a trait when
independent implementations genuinely need substitution. Do not create a trait for one concrete
type without a boundary or testing need.

### Liskov substitution

Every implementation must preserve the trait's documented preconditions, results, error semantics,
side effects, ordering, and ownership rules. An implementation that panics or silently weakens a
contract is not substitutable.

### Interface segregation

Define small traits from the consumer's needs. Avoid "manager" interfaces and traits whose users
depend on only one method. Keep read and mutation capabilities separate when that makes authority
and testing clearer.

### Dependency inversion

Put abstractions at volatile boundaries such as clocks, storage, external services, media backends,
and platform integration. Keep stable internal computation concrete. Injecting every helper behind
a trait increases complexity without improving isolation.

## Design smells worth reporting

- Boolean parameters whose meaning is unclear at the call site.
- A function that mutates distant state in addition to its stated result.
- Repeated match trees or validation rules representing the same domain decision.
- Public types that permit invalid states which every caller must re-check.
- Error swallowing, unconditional fallback, or logging followed by false success.
- A new abstraction that is larger or harder to name than the duplication it replaces.
- Cross-crate dependency direction that bypasses `avbridge -> core -> ui`.

## Review questions

1. Is the behavior and failure contract visible from the API?
2. Can invalid state be made unrepresentable or validated once at a boundary?
3. Is each abstraction justified by current variation?
4. Does the change preserve crate and ownership boundaries?
5. Is the simplest correct implementation also easy to test and observe?
