# ADR-017 — Recalibrate the Composite as a Gating-Pressure Score

**Status:** Accepted — Implemented

---

## Context

ADR-010's commercial-value model (as first implemented) computed the composite
as a weighted **average** of seven "positive" dimensions, with `support_burden`
applied as a penalty around a neutral midpoint of 5.0. Two problems surfaced
when running real repositories (mini-redis, midstream):

1. **Adoption and strategic importance pulled the wrong way.** Both raised the
   composite (and therefore the gating tier), but per ADR-010 high adoption /
   strategic importance mean "keep this open for the community flywheel" — they
   should pull *toward* Open, not toward a higher tier.
2. **Everything compressed into one band.** Averaging eight 0–10 dimensions
   pushed disparate modules into a narrow ~5–6 range, so a commodity library and
   a crown-jewel IP crate both mapped to `ProTier`. On midstream all seven crates
   landed in `pro_tier`, making the tier output non-actionable.
3. **`support_burden` semantics were inverted.** ADR-010 describes heavy support
   burden as an *enterprise-tier signal* (justifies support contracts), but the
   implementation subtracted it, lowering the composite.

## Decision

Redefine the composite as a 0–10 **gating-pressure score** ("how gateable is
this"), not a generic average:

- **Gate-positive dimensions** form the gate pressure (weighted average):
  `enterprise_buyer_value`, `commercial_leverage`, `competitive_sensitivity`,
  `operational_value`, and `support_burden`.
- **Openness dimensions** discount the score: `adoption_value` and
  `strategic_importance` reduce gate pressure by up to 60%
  (`MAX_OPENNESS_DISCOUNT`), reflecting "keep open for adoption".
- `security_sensitivity` is removed from the tier computation; it remains a
  review/risk signal surfaced in the report, not a gating driver.

Formula: `composite = gate_pressure * (1 - (openness / 10) * 0.6)`, clamped to
`[0, 10]`. The tier bands (ADR-010) and license-risk override are unchanged.

## Consequences

**Positive:**
- Low-value modules score low → `Open`; high-IP, low-adoption modules score high
  → `ProTier`/`EnterpriseTier`; high-adoption commodities are discounted back
  toward `Open`. On midstream the tiers now spread `open` (temporal-compare,
  xtask) → `source_available` (scheduler, quic, attractor) → `pro_tier`
  (neural-solver, strange-loop), matching the analyst narrative.
- `support_burden` now aligns with ADR-010's stated intent.

**Negative / Trade-offs:**
- An all-average module (all dims 5.0) now scores ~3.5 rather than 5.0 — the
  midpoint is no longer "neutral", it leans slightly open. The corresponding
  acceptance test was updated.
- Strategically-important crown-jewel crates are discounted by their own
  strategic-importance score, so they may land in `ProTier` rather than
  `EnterpriseTier`. This is an intentional synthesis of the competitive-vs-
  community tension; the agent's narrative carries the finer recommendation.

## Alternatives Considered

- **Keep averaging, re-sign weights.** Making adoption/strategic weights negative
  in the average still left everything compressed near the midpoint; it did not
  produce real separation.
- **Let the agent's `estimated_tier` override the computed tier.** Rejected as
  the default: the deterministic score should be independently meaningful. The
  agent's narrative and per-module rationale remain the qualitative layer on top.

## Relationships

Amends ADR-010 (commercial value scoring model). Builds on ADR-016 (which added
the real per-dimension `commercial_score` that makes this calibration meaningful).
