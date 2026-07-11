# Counter Example

This example is the first public proof for the clean RunenUI architecture.

The counter example must demonstrate the full typed UI loop without using legacy compiler, program, artifact, ECS, or route-bridge machinery.

## Target behavior

The example will eventually prove:

* application-owned `Counter` state
* application-owned `CounterAction`
* `update(&mut Counter, CounterAction)`
* `root(&Counter) -> Element<CounterAction>`
* typed button actions
* a counter screen
* a win screen when the count reaches the configured threshold
* reset back to the counter screen
* headless runtime execution
* inspectable trace output
* published surface-frame data from tight root constraints and deterministic measurement
* aligned style and layout diagnostics from the same publication
* exactly one deterministic measurement per text or button label

## Intended shape

```rust
#[derive(Default)]
struct Counter {
    count: i32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CounterAction {
    Increment,
    Decrement,
    Reset,
}

fn update(counter: &mut Counter, action: CounterAction) {
    match action {
        CounterAction::Increment => counter.count += 1,
        CounterAction::Decrement => counter.count -= 1,
        CounterAction::Reset => counter.count = 0,
    }
}

fn root(counter: &Counter) -> Element<CounterAction> {
    if counter.count >= 10 {
        win_screen(counter)
    } else {
        counter_screen(counter)
    }
}
```

## Non-goals

This example must not introduce:

* renderer-specific code
* windowing code
* ECS host ownership
* route-string action resolution
* schema payload action dispatch
* compiler/program/artifact dependencies
* legacy crate imports

The first implementation should run headlessly and prove the runtime contract before any renderer backend is added.

The current Counter uses the small row/column contract: finite cross-axis constraints propagate through content boxes, the main axis remains intrinsic, and overflow is diagnostic only. The example does not clip or scroll.

For workspace-wide dependency rules, see [dependency-map](../../docs/dependency-map.md).

For implementation maturity, see [status-map](../../docs/status-map.md).
