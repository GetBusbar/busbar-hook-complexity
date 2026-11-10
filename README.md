# busbar-hook-complexity

A text-classification routing hook using an admitted, bounded decisions-plane child request. The host retains caller identity, credentials, budgets, cancellation and parent correlation. The hook receives only the normalized classification answer.

The default threshold is 0.7, inclusive. Missing confidence abstains; `missing_confidence: accept` explicitly permits a classifier without confidence. Scores must be finite within `[0,1]`. Complete normalized probabilities are validated when present; this implementation never invents confidence from scores or probabilities. Unknown labels, unsupported schemas, unavailable classifiers, timeouts, unsupported projections and oversized input all abstain with a reason in the plugin log.

Accepted tiers prioritize only models already present in the authorized candidate set. Normal ranking operates within the preferred group, then ordinary fallback candidates remain available. No candidate is added or made exempt from admission policy. Empty intersections abstain. Restriction mode and cross-request classification caching are not provided.

## Configuration

Attach this hook to the parent LLM pool. The deployment supplies both `prompt: ro` and `request_text_complete` in `signals`. The LLM plane must explicitly attest a complete text projection; an absent or false signal abstains. The hook never reads raw request bodies. Tool-bearing requests abstain. The latest user message and at most `previous_messages` earlier user/assistant messages are classified; a window exceeding the byte bound is skipped rather than silently truncated.

```yaml
hooks:
  complexity:
    kind: gate
    module: complexity
    prompt: ro
    user: no
    signals: [request_text_complete]
    timeout_ms: 1500
    on_error: weighted
    child_dispatch:
      revision: 1
      plane: decisions
      operation: POST
      target: /busbar/decisions/v1/classify
      target_binding: classifier
      max_input_bytes: 16384
      max_output_bytes: 65536
      max_children: 1
      timeout_ms: 1000
      max_budget_micros: 1000000
    settings:
      classifier_model: classifier
      mode: prefer
      min_confidence: 0.7
      missing_confidence: abstain
      previous_messages: 2
      max_input_bytes: 16384
      tiers:
        - label: easy
          description: Short requests requiring little reasoning
          examples: [Greet the user]
          candidates: [small]
        - label: hard
          description: Requests requiring several reasoning steps
          candidates: [large]
```

`plane` names the deployed decisions instance, and `target_binding` must match `classifier_model`. The host validates that the configured plane, claim and binding exist. Set the budget and deadline to the operator's limits. The hook has no independently selected network destination, credentials or provider API settings. A host with no child grant cannot classify.

Settings are immutable for an opened hook instance; change them through configuration reload, with a new child grant revision. An identical configure replay is acknowledged with a complete checkpoint; a push changing settings is refused. The host memoizes the single child result throughout the admitted parent request, including retries and failover; repeated polls use the same completion handle and owned payload. A child that incurred usage is billed even if its answer is rejected or times out.

## Packaging

The `hook-complexity` rlib and `hook-complexity-plugin` cdylib share one door. Build the dropped-in image with `cargo build -p busbar-hook-complexity-plugin --features dropped-in --release`. Runtime Busbar dependency is only `busbar-contract`.

The pinned SDK provides child dispatch, explicit text-projection completeness and grouped candidate preference. Local development overrides do not belong in shipped manifests. No live-provider quality, latency, savings or accuracy claim is made by these source fixtures.
