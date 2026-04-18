# Stateless L7 Authz at the Edge: Self-Routing Tokens with Obfuskey

A few weeks ago I started with a question that sounded like clickbait: *"Are Snowflake IDs leaking data?"* The honest answer is "not really — but they are leaking **structure**, and that's enough to matter." Over the course of poking at it, what started as a one-line fix turned into a small architecture: stateless Layer 7 authorization and routing at the edge, with Obfuskey carrying compact structured tokens, HMAC carrying integrity, and a JWT carrying identity.

This post is the shape of that design and the honest tradeoffs behind it.

## What Snowflake IDs actually expose

A Snowflake ID is a 64-bit integer with three fields packed into it:

- **41 bits** of millisecond timestamp
- **10 bits** of worker/machine ID
- **12 bits** of sequence counter

Anyone with the ID can recover all three:

```python
def decode_snowflake(sid: int, epoch: int = 1288834974657):
    ts = (sid >> 22) + epoch
    worker = (sid >> 12) & 0x3FF
    seq = sid & 0xFFF
    return ts, worker, seq
```

This is by design — the encoding is public. The problem isn't secrecy; it's **exposure**. When a Snowflake ends up in a URL, a competitor can:

- Chart your hourly request volume by bucketing timestamps
- Estimate signup or checkout rates by watching the sequence counter
- Enumerate neighbors by subtracting from an ID they've already seen
- Fingerprint your infrastructure topology from worker IDs

None of that is a CVE. None of it is "data" in the PII sense. It's **signal**, and signal is what competitive scrapers, traffic analysts, and abuse pipelines actually consume.

## What doesn't fix it

A few patterns people reach for that don't actually help:

- **Switching to UUIDv7.** Same 48-bit timestamp prefix, same traffic analysis window.
- **Base62-encoding the raw ID.** You made it shorter; the bits are identical.
- **"Nobody will bother decoding it."** Someone writes a gist in an afternoon and you're back to square one.

The real fix is breaking the visible correlation between adjacent IDs, and — where possible — not putting routing/identity structure in outward-facing tokens at all.

## What Obfuskey actually is

[Obfuskey](https://github.com/bnlucas/obfuskey) is a small library that turns an integer into a short string key by multiplying by a large prime modulo `alphabet_size ^ key_length`, then base-converting against the alphabet. It's reversible with the same alphabet and multiplier.

```python
from obfuskey import Obfuskey, alphabets

obf = Obfuskey(alphabets.BASE62, key_length=10)
obf.get_key(1234567890)      # e.g. 'Fb4ADhxJs3'
obf.get_value('Fb4ADhxJs3')  # 1234567890
```

Two important things to be clear about:

- **It's obfuscation, not encryption.** The multiplier is the only secret, and it's recoverable with a small number of known (plaintext, key) pairs. Treat it as "will eventually leak."
- **The avalanche is the actual feature.** Adjacent inputs produce completely unrelated-looking keys, because prime-multiplication over a large modulus spreads differences across the whole output. Two timestamps one millisecond apart look nothing alike.

That second property is exactly the anti-enumeration, anti-traffic-analysis win we need. A scraper watching URLs can't tell the difference between "this is 1ms after that one" and "this is a totally different user from a different region six months later."

## Configuration entropy is the real knob

The default Obfuskey configuration — standard BASE62, default multiplier — is the weakest possible deployment. But Obfuskey's alphabet isn't a character *set*, it's a character **permutation**. A 62-character alphabet has 62! ≈ 3×10⁸⁵ possible orderings. And nothing requires you to use a standard set at all:

```python
import random
from obfuskey import Obfuskey, alphabets

alpha = list(alphabets.BASE62)
del alpha[10]          # now 61 characters
random.shuffle(alpha)  # arbitrary order
alpha = ''.join(alpha)

obf = Obfuskey(alpha, key_length=20)
```

You now have a 61-character alphabet in a permutation that matches no published default, with arbitrary digit-value assignments. An attacker trying to "guess your alphabet" isn't picking from a shortlist anymore.

This is the honest framing: **Obfuskey's strength comes from configuration entropy, not cryptography.** Default alphabet + default multiplier + public schema = weak. Custom alphabet + custom multiplier + private schema = a real barrier, scaling with how little you telegraph about your setup. The library gives you every knob; it's on the deployment to use them.

## Packing structure with Obfusbit

Obfuskey has a sibling, Obfusbit, that packs a bit-level schema into a single integer before obfuscating:

```python
from obfuskey import Obfuskey, Obfusbit

schema = [
    {"name": "timestamp",   "bits": 42},
    {"name": "region_id",   "bits": 4},
    {"name": "cell_id",     "bits": 4},
    {"name": "user_id",     "bits": 32},
    {"name": "resource_id", "bits": 32},
    {"name": "version_id",  "bits": 8},
    {"name": "hmac",        "bits": 64},
]

obf = Obfuskey(custom_alphabet, key_length=32)
packer = Obfusbit(schema, obfuskey=obf)
```

At first glance this *looks* like the same mistake Snowflake makes — exposing structure in the token. But there's a crucial difference: **the schema is private.** An attacker who recovers the multiplier and alphabet still sees an opaque integer. They don't know which bits are timestamp, which are user_id, or even that it's structured at all.

And structured tokens enable the real pattern this post is about.

## Self-routing tokens

The question I was actually trying to answer: *can you get Layer 7 routing with something closer to Layer 4 efficiency?*

The usual L7 path looks like:

```
request → parse → look up routing (control plane / service discovery / DB) → authz service → backend
```

The cost isn't the parsing; it's the lookups. Every hop adds milliseconds and stateful dependencies. L4 is fast because it's stateless and information-dense — TCP/IP headers contain everything you need to route.

The self-routing-token pattern pulls the same trick at L7: **put the routing intelligence in the request itself.** If the token contains `region_id`, `cell_id`, `user_id`, and `resource_id`, the edge proxy knows exactly where to send the traffic and who's allowed to, without consulting anything.

Put concretely, the user presents two things:

1. **A JWT (or session cookie)** — standard identity proof, signed by an auth service.
2. **An Obfuskey token in a header** — carrying `(region, cell, user_id, resource_id, version, hmac)`.

An Envoy `ext_proc` filter at the edge does:

```
1. Verify JWT  → extract authenticated user_id
2. Unpack Obfuskey header token
3. Verify HMAC field over the other fields
4. Check token.user_id == jwt.sub
5. Check token.resource_id == URL path resource_id
6. Route to (region_id, cell_id) backend
```

Any mismatch → **403 at the edge**. No application hit. No database hit. No authz microservice call. The edge made a full L7 authorization decision from data carried in the request, with no lookups of its own.

This is "Layer 7 routing without Layer 7 lookup overhead." Not literally L4 speed — you're still doing modular multiplication and HMAC verification — but bounded, stateless, single-digit microseconds, deterministic across nodes.

## Why the three layers are all necessary

It's tempting to collapse this into one primitive. Don't. Each layer does one job:

| Layer | Carries | Secret | Failure mode if skipped |
|---|---|---|---|
| **JWT** | Identity | Auth service signing key | Anyone can claim to be any user |
| **Obfuskey** | Routing + scope | Multiplier + alphabet + schema | Structure is plainly readable |
| **HMAC** | Integrity | HMAC key | Multiplier leak = forgeable tokens |

Obfuskey alone is reversible — if the multiplier ever leaks (treat this as "when," not "if"), forgery becomes possible unless there's an independent integrity layer. HMAC alone gives you integrity but no compactness and no structure. JWT alone gives you identity but is much larger and doesn't encode routing.

The combination gets you: **small tokens, cheap edge decisions, integrity that survives obfuscation compromise, and identity that survives token leakage.**

## Putting the HMAC inside the schema

You can reserve an HMAC field in the Obfusbit schema rather than carrying it separately. There are three non-obvious things to get right:

**1. You can't HMAC the full packed integer and include the result inside it.** Chicken-and-egg. The pattern:

```python
# Compute HMAC over all non-HMAC fields
values_for_mac = {k: v for k, v in values.items() if k != "hmac"}
mac_input = canonical_encode(values_for_mac)
mac_bits = truncate(hmac_sha256(key, mac_input), 64)

values["hmac"] = mac_bits
token = packer.pack(values, obfuscate=True)
```

Verify by unpacking, zeroing the HMAC field, recomputing, comparing.

**2. HMAC everything the edge trusts.** If you skip `region_id`, an attacker with the multiplier can rewrite routing without breaking the MAC. Include every field that influences an authorization or routing decision.

**3. Truncation budget.** 64-bit truncated HMAC gives ~2⁶³ forgery effort. Fine for short-lived tokens, marginal for resource URLs that live for years. 96 bits is the sweet spot if you can spare them — still fits comfortably in a 22-character BASE62 token.

And a nice property falls out: the HMAC key stays central and small, while the multiplier can live at every edge node without existential risk. Multiplier compromise stops meaning "forge anything" and starts meaning "read structure but can't modify it" — a much more survivable failure mode.

## The extproc, in Rust

Envoy's `ext_proc` filter is just a bidirectional gRPC stream — Envoy sends `ProcessingRequest` messages, your service answers with `ProcessingResponse`. For this pattern we only care about `request_headers`: we inspect them, decide, and either let the request continue or short-circuit with a 403.

Skipping the `tonic` plumbing, the config and decision logic look like this:

```rust
use hmac::{Hmac, Mac};
use num_bigint::BigUint;
use obfuskey::{Obfusbit, UnpackDataBig};
use sha2::Sha256;
use std::collections::HashMap;

type HmacSha256 = Hmac<Sha256>;

pub struct EdgeAuthz {
    packer: Obfusbit,
    hmac_keys: HashMap<u8, Vec<u8>>, // version_id -> key
    jwt_decoding_key: jsonwebtoken::DecodingKey,
}

#[derive(Debug)]
enum Decision {
    Allow { region: u8, cell: u8 },
    Deny(&'static str),
}

impl EdgeAuthz {
    fn check(
        &mut self,
        token_header: &str,
        jwt_header: &str,
        url_resource_id: u64,
    ) -> Decision {
        // 1. Verify JWT, extract authenticated user_id
        let sub = match verify_jwt(&self.jwt_decoding_key, jwt_header) {
            Ok(sub) => sub,
            Err(_) => return Decision::Deny("invalid jwt"),
        };

        // 2. Unpack Obfuskey token (reversible, not yet trusted)
        let fields = match self
            .packer
            .unpack_big(UnpackDataBig::Key(token_header), true)
        {
            Ok(f) => f,
            Err(_) => return Decision::Deny("malformed token"),
        };

        // 3. Recompute HMAC over every field except the hmac slot itself
        let version = to_u64(&fields["version_id"]) as u8;
        let Some(key) = self.hmac_keys.get(&version) else {
            return Decision::Deny("unknown token version");
        };
        let expected = compute_hmac(key, &fields);
        let got = to_u64(&fields["hmac"]);
        if expected != got {
            return Decision::Deny("bad hmac");
        }

        // 4. Bind token to authenticated identity
        if to_u64(&fields["user_id"]) != sub {
            return Decision::Deny("user mismatch");
        }

        // 5. Bind token to URL
        if to_u64(&fields["resource_id"]) != url_resource_id {
            return Decision::Deny("resource mismatch");
        }

        Decision::Allow {
            region: to_u64(&fields["region_id"]) as u8,
            cell: to_u64(&fields["cell_id"]) as u8,
        }
    }
}

fn compute_hmac(key: &[u8], fields: &HashMap<String, BigUint>) -> u64 {
    let mut mac = <HmacSha256 as Mac>::new_from_slice(key).unwrap();
    for name in ["timestamp", "region_id", "cell_id",
                 "user_id", "resource_id", "version_id"] {
        mac.update(&fields[name].to_bytes_be());
        mac.update(&[0xFF]); // length delimiter
    }
    let tag = mac.finalize().into_bytes();
    u64::from_be_bytes(tag[..8].try_into().unwrap())
}

fn to_u64(v: &BigUint) -> u64 {
    v.iter_u64_digits().next().unwrap_or(0)
}
```

The `ext_proc` handler itself is mostly protocol glue — pull the headers out of `ProcessingRequest::RequestHeaders`, call `check`, and translate the `Decision`:

```rust
async fn on_request_headers(
    &mut self,
    headers: &HeaderMap,
) -> ProcessingResponse {
    let token = headers.get("x-resource-token").map(str::to_owned);
    let jwt   = headers.get("authorization").map(str::to_owned);
    let path  = headers.get(":path").unwrap_or_default();

    let Some((token, jwt, resource_id)) = extract(token, jwt, path) else {
        return immediate_403("missing credentials");
    };

    match self.authz.check(&token, &jwt, resource_id) {
        Decision::Allow { region, cell } => {
            continue_with_routing_header(region, cell)
        }
        Decision::Deny(reason) => immediate_403(reason),
    }
}
```

The routing piece is where the "self-routing" property cashes out: instead of consulting a routing table, the handler sets a header like `x-upstream-cell: 3.7` that your Envoy cluster config maps directly to the right upstream. The extproc never talks to a database, a cache, an authz service, or a control plane. Every decision is made from data carried in the request.

### What the edge actually pays per request

Rough back-of-envelope for the Rust path:

- JWT HS256 verify: ~2–5 µs
- Obfuskey unpack (BigUint at ~200 bits): ~3–8 µs
- HMAC-SHA256 of ~30 bytes: ~1 µs
- Three integer comparisons: lost in noise

Single-digit microseconds of compute per request, no I/O, no locks, no shared state beyond a small key map that's read-only at steady state. That's the "L7 routing at close-to-L4 cost" claim made concrete — not literally L4, but bounded, deterministic, and cheap enough that every edge node can do it without scaling concerns.

## Rotation

Long-lived tokens and rotating secrets are a pain. The `version_id` field in the schema handles it gracefully:

- Every issued token carries the version of the multiplier/HMAC key pair it was signed with
- The edge keeps the current version + a short tail of previous versions
- New tokens are issued with the current version
- Old tokens validate against their own version until you retire it

This lets you rotate without invalidating every outstanding URL, at the cost of keeping a small rotation window of keys live at every edge. A reasonable tradeoff.

## What this actually gives you

Pulling it together, the pattern provides:

- **Stateless edge authorization** — no DB hit, no authz microservice call, no session store lookup
- **Self-routing data** — every edge node makes the same decision from the same inputs
- **Integrity surviving obfuscation compromise** — HMAC key and multiplier are separate concerns
- **Compact URLs** — 20–25 characters for 128+ bits of structured data
- **Graceful rotation** — `version_id` lets you roll keys without breaking live tokens
- **Small pockets** — 4 bits region × 4 bits cell × 32 bits user = 16 regions × 16 cells × 4.3B users per cell, which is enough for most products without the pain of a 64-bit global user ID

## When *not* to use this

- **If the token must stand alone** as a capability (shareable links, offline verification, no JWT in the picture), use something signature-based like PASETO or Branca. Obfuskey + HMAC can carry its own integrity but isn't built to be a general-purpose signed token.
- **If compactness doesn't matter.** Just use a signed JWT with routing claims. Slightly larger, battle-tested, integrates with everything.
- **If you need forward-secrecy or repudiation properties.** Symmetric HMAC doesn't give you those. Go asymmetric.

## The honest takeaway

Snowflake IDs don't leak data in the sensitive sense. They leak **shape** — timestamps, sequence counters, worker IDs — and that shape is valuable to competitors, scrapers, and enumeration pipelines. The fix isn't cryptography; it's making the shape unreadable to observers and binding it to identity for validators.

Obfuskey's role in that is narrow and well-defined: compactness, avalanche, and structured packing. It isn't a signature scheme, and claiming otherwise does the library a disservice. But paired with HMAC for integrity and a JWT for identity, it becomes the enabling piece of a genuinely interesting architectural pattern — one where edge proxies make full Layer 7 decisions at almost Layer 4 cost.

Stop leaking structure. Put structure where it helps you — in tokens the edge can read — and keep it out of URLs anyone can.
