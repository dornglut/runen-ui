# External renderer-neutral consumer conformance fixture

This unpublished workspace package is a genuine downstream consumer of RunenUI's ordinary public renderer-neutral scene products. It exists to prove M6 protocol portability without becoming a production renderer backend.

The library depends only on `runenui_core` and `runenui_runtime`. It does not use `runenui_testing`, private runtime seams, concrete widgets, semantic roles, mounted/layout storage, native host types, resource providers, decoding/shaping, or backend realization.

Its consumer deterministically records complete public `PaintPublication` and `HitTestScene` facts, checks declared resource capabilities, distinguishes exact predecessor matches from full resynchronization, and rebuilds every returned scene snapshot from the supplied public products. It intentionally owns no hidden RunenUI scene cache.
