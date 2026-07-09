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
* published surface-frame data

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

For workspace-wide dependency rules, see [dependency-map](../../docs/dependency-map.md).

For implementation maturity, see [status-map](../../docs/status-map.md).
