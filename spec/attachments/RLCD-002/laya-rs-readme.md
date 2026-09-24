# laya-rs

A from-scratch Rust reimplementation of [Laya](https://laya.convaiinnovations.com/), Convai
Innovations' sub-35ms, non-autoregressive "System 1" decision engine: give it a **state**
(text, email, ticket, or JSON) and typed **questions** (`choice` / `score` / `noul`), and it
returns typed answers with calibrated probabilities in a single bidirectional forward pass —
no text generation, nothing to parse, nothing to hallucinate.

This port targets the published `convaiinnovations/laya` checkpoint family (ModernBERT-large /
mmBERT-base backbones + a from-scratch decision head) and reimplements the full stack natively
on [candle](https://github.com/huggingface/candle), with no PyTorch/Python dependency at
inference time:

- **ModernBERT encoder from scratch** — RoPE with per-layer-type theta, alternating
  global/sliding-window attention, GeGLU MLP — config-driven, so it runs both the 421M
  ModernBERT-large and 322M mmBERT-base checkpoints.
- **Decision head** — type embedding, 2-layer transformer, option-marker scoring, act head —
  vectorized and autograd-friendly, so the same code path serves both fast inference and training.
- **Typed-question schema builder** — `choice` / `score` / `noul` rendering and token-budgeted
  sequence construction, matching the original's `[CLS] ... [SEP] [MASK] opt0 [MASK] opt1 ... [SEP]
  state [SEP]` layout and truncation rules.
- **Per-(question-type, option-count) temperature calibration** at inference time, plus a
  post-hoc temperature-fitting utility.
- **RLCD training loop** — REINFORCE with Gaussian-noise exploration and a group-mean baseline
  (GRPO-style) over the same strictly-proper scoring rule (log + spherical + ranked probability
  score) the original uses as its reward, with TD(λ) bootstrapping for multi-turn episodes.
- **Language routing** — a fast 22-script Unicode detector for the "wrong alphabet" case, backed
  by [whichlang](https://github.com/quickwit-oss/whichlang) for language identification within
  Latin script, so routing never has to guess from model confidence (an English-only checkpoint
  can be confidently wrong on scripts it can't read).

Comparison against the reference `jev` API and a `gliner` baseline on a typed-decisions fixture:
https://gist.github.com/framp/82a9973988cc41a8b552cb7850b70259

## Serving

`laya` can run as a standalone HTTP server exposing the open [Jev/Simple-Jev v1
classifier](docs/jev-server/protocol-reference.md) protocol — a checkpoint, a
listener, and a handful of endpoints:

```bash
laya serve
```

With no arguments this downloads the default `typed-decisions` checkpoint into
`~/.cache/laya-rs` on first use, loads it, and binds `127.0.0.1:8000`. Point it
at a specific checkpoint with `--model` (a local directory) or pick a different
variant with `--model-variant multilingual`. The model name the server reports
and that clients must echo in each request's `model` field is the value you
supplied — the variant key (default `typed-decisions`) or the `--model` path.

```bash
# serve the default (English) checkpoint on 127.0.0.1:8000
laya serve

# serve a specific local checkpoint directory, on all interfaces, port 9000
laya serve --model /path/to/laya-typed-decisions --host 0.0.0.0 --port 9000
```

| Endpoint | Description |
| --- | --- |
| `POST /v1/classifier` | Answer a batch of typed questions against one state or chat history |
| `POST /v1/systemone` | Exact alias of `/v1/classifier` |
| `GET /health` | `{"status":"ready","model":"<loaded model>"}` — readiness, no inference |
| `GET /openapi.json` | The OpenAPI 3.1 spec for the endpoints above |

Questions execute **serially** against the model: one forward in flight plus a
bounded admission queue (16 waiting slots by default). A request that finds the
queue full gets a `429` with a `Retry-After` header rather than being dropped.
Each request is capped at 100 questions and a 1 MiB body. Ctrl-C or SIGTERM
shuts down gracefully — the listener stops accepting, in-flight forwards run to
completion, then the process exits.

A full request/response:

```bash
curl -s http://127.0.0.1:8000/v1/classifier \
  -H 'Content-Type: application/json' \
  -d '{
    "model": "typed-decisions",
    "state": "We were billed twice for March. Please refund the duplicate.",
    "questions": {
      "intent": {
        "type": "choice",
        "instructions": "What does the customer want?",
        "criteria": { "refund": "money back", "cancel": "end the account", "other": null }
      },
      "urgency": {
        "type": "score",
        "instructions": "How urgent is this?",
        "criteria": [ "not urgent", "soon", "blocking" ]
      }
    }
  }'
```

```json
{
  "model": "typed-decisions",
  "answers": {
    "intent": {
      "type": "choice",
      "choice": "refund",
      "confidence": 0.82,
      "probabilities": { "refund": 0.82, "cancel": 0.07, "other": 0.11 }
    },
    "urgency": {
      "type": "score",
      "score": 1.66,
      "confidence": 0.72,
      "probabilities": { "0": 0.06, "1": 0.23, "2": 0.72 },
      "legend": { "0": "not urgent", "1": "soon", "2": "blocking" }
    }
  },
  "usage": { "input_tokens": 78, "output_tokens": 0 }
}
```

`state` accepts a string, a JSON object, or a JSON array; `messages` accepts a
text-only chat history (`role` + `content`) as an alternative — send exactly one
of the two. The three question types (`choice`, `score`, `noul`) and the full
`422` error envelope (`error.details[]`) are documented in
[`docs/jev-server/`](docs/jev-server/).

`laya serve` options:

| Flag | Env | Default | Description |
| --- | --- | --- | --- |
| `--model <DIR>` | `LAYA_MODEL` | — | A local checkpoint directory, bypassing `--model-variant`/`--models-root` |
| `--model-variant <KEY>` | — | `typed-decisions` | Which checkpoint to use when `--model` isn't given |
| `--models-root <DIR>` | `LAYA_MODELS_ROOT` | — | Root of a checkpoint family, checked before downloading |
| `--host <ADDR>` | — | `127.0.0.1` | Bind address |
| `--port <N>` | — | `8000` | Bind port |
| `--max-model-len <N>` | `LAYA_MAX_MODEL_LEN` | checkpoint `max_len` | Per-question sequence cap (clamped to the checkpoint's native max) |
| `--max-request-branches <N>` | `LAYA_MAX_REQUEST_BRANCHES` | `100` | Max questions per request |
| `--max-queued <N>` | `LAYA_MAX_QUEUED` | `16` | Admission-queue depth on top of the in-flight forward |

## Installation

macOS (Apple Silicon) and Linux, via Homebrew:

```sh
brew tap apiplant/tap
brew install apiplant/tap/laya-rs
```

Arch Linux, via the signed pacman repository at `apiplant.github.io/pacman`
(one-time setup, then `pacman -Sy`/`-Syu` picks up new releases):

```sh
curl -sSfL https://apiplant.github.io/pacman/apiplant.gpg -o /tmp/apiplant.gpg
keyid=$(gpg --show-keys --with-colons /tmp/apiplant.gpg | awk -F: '/^pub:/ { print $5; exit }') && sudo pacman-key --add /tmp/apiplant.gpg && sudo pacman-key --finger "$keyid" && sudo pacman-key --lsign-key "$keyid"
printf '\n[apiplant]\nSigLevel = Required DatabaseOptional\nServer = https://apiplant.github.io/pacman/$arch\n' | sudo tee -a /etc/pacman.conf > /dev/null
sudo pacman -Sy laya-rs
```

Debian/Ubuntu, via the signed apt repository at `apt.apiplant.com` (one-time
setup, then `apt upgrade` picks up new releases):

```sh
curl -sSfL https://apt.apiplant.com/apiplant-archive-keyring.gpg | sudo tee /usr/share/keyrings/apiplant.gpg > /dev/null
echo "deb [signed-by=/usr/share/keyrings/apiplant.gpg] https://apt.apiplant.com stable main" | sudo tee /etc/apt/sources.list.d/apiplant.list > /dev/null
sudo apt update && sudo apt install laya-rs
```

Or download the archive, `.deb`, or `.pkg.tar.zst` for your platform from the
[releases page](https://github.com/apiplant/laya-rs/releases) and install it
directly — the plain archive needs no installation at all, `laya` is static
enough to run from anywhere. On Linux x86_64 with an Ampere-or-newer NVIDIA
GPU, grab the `laya-rs-flash-attn-*-x86_64-unknown-linux-gnu.tar.gz` archive
instead for CUDA + flash-attention-accelerated inference (needs a host driver
compatible with the CUDA toolkit it was built against).

As a Rust library, or to build the CLI from source, via crates.io:

```sh
cargo add laya-rs         # as a library dependency
cargo install laya-rs     # for the laya binary
cargo install laya-rs --features flash-attn  # with CUDA + flash-attn support
```

| Platform | Ships as |
| --- | --- |
| macOS (Apple Silicon) | archive, Homebrew |
| Linux x86_64 | archive, `.deb` + apt repo, Arch package + pacman repo, Homebrew |
| Linux x86_64, CUDA + flash-attn | archive, `.deb` + apt repo, Arch package + pacman repo |
| Linux aarch64 | archive, `.deb` + apt repo, Homebrew |

No macOS Intel build: only Apple Silicon (`aarch64-apple-darwin`) and Linux
(`x86_64`/`aarch64`) are supported.

See [`packaging/README.md`](packaging/README.md) for how these packages are
built and published.

## Library

`cargo add laya-rs` pulls in the `laya` crate (native target; the `wasm32-unknown-unknown`
target instead exposes `laya::wasm::WasmAgent`, the same interface wrapped for
`wasm-bindgen` — see `website/src/lib/laya.ts` for how the browser demo drives it). The
surface is small: load an [`RLAgent`](src/agent.rs) from a checkpoint directory, build typed
[`Question`](src/schema.rs)s, and get back typed [`Answer`](src/agent.rs)s.

```rust
use serde_json::json;
use laya::{Answer, QType, Question, RLAgent};

fn main() -> anyhow::Result<()> {
    // A checkpoint directory downloaded from Hugging Face (convaiinnovations/laya,
    // -typed-decisions, or -multilingual) — rl_agent_config.json, tokenizer/, encoder/, model.safetensors.
    let agent = RLAgent::load("/path/to/laya-typed-decisions")?;

    let state = json!("We were billed twice for March. Please refund the duplicate.");
    let question = Question {
        qtype: QType::Choice,
        instructions: "What does the customer want?".to_string(),
        choice_criteria: vec![
            ("refund".to_string(), None),
            ("cancel".to_string(), None),
            ("other".to_string(), None),
        ],
        score_criteria: vec![],
        noul_true: None,
        noul_false: None,
    };

    // Batched: pass as many (id, Question) pairs as you like in one forward pass.
    let answers = agent.system_one(&state, &[("intent".to_string(), question)])?;
    for (id, answer) in answers {
        match answer {
            Answer::Choice { choice, probabilities, confidence, act_probability } => {
                println!("{id}: {choice} (confidence={confidence:.3}, act_p={act_probability:.3})");
                for (option, p) in probabilities {
                    println!("    {option}: {p:.3}");
                }
            }
            Answer::Score { score, confidence, .. } => println!("{id}: {score:.2} (confidence={confidence:.3})"),
            Answer::Noul { noul, .. } => println!("{id}: {noul:.3}"),
        }
    }
    Ok(())
}
```

`Answer` is a plain enum, not `Serialize` — `laya::agent::answer_to_json` turns one into the
same `{"type": "choice"|"score"|"noul", ...}` JSON shape the CLI's `answer` subcommand and the
wasm bindings emit, if that's more convenient than matching on it directly.

Other pieces of the public API, all optional depending on what you need:

- `laya::route` / `laya::Checkpoint` — the English-vs-multilingual language router, so you can
  pick a checkpoint from a state string before loading (native only; not exposed to wasm since
  the browser demo picks a checkpoint from the UI instead).
- `laya::model_path` / `laya::download` — the CLI's own checkpoint resolution: given a variant
  key, find it under a local family root or fetch it into `~/.cache/laya-rs` (native only).
- `RLAgent::load_from_bytes` — the same load path as `RLAgent::load`, but from in-memory file
  contents instead of a filesystem path (what the wasm bindings use, since there's no filesystem
  in a browser tab).
- `laya::RlcdConfig` / `laya::Trainer` — the RLCD training loop (`Trainer::load` +
  `Trainer::train_step`/`train_jsonl`), for fine-tuning a checkpoint rather than just running it.

## Performance

CPU by default. For GPU inference:

```bash
cargo build --release --features cuda
```

This runs in F16 (matching the original Python implementation's own default precision, and what
actually engages the GPU's tensor cores).

An optional `flash-attn` feature additionally swaps the encoder's and decision head's attention
for a fused flash-attention kernel, and runs the whole stack *unpadded* — real tokens from every
row are packed into one flat sequence, with row boundaries passed to flash-attn's varlen kernel,
so padding is never computed on or attended to:

```bash
CUDA_COMPUTE_CAP=<your GPU's compute capability, e.g. 89 for Ada/RTX 40xx> \
  cargo build --release --features flash-attn
```

The first build compiles NVIDIA's cutlass headers against flash-attention's CUDA kernels (a few
minutes, cached afterwards). `CUDARC_CUDA_VERSION` is pinned in `.cargo/config.toml` since
`cudarc` doesn't yet recognize newer CUDA toolkits without the override.

On an RTX 4090, answering a 16-question typed-decisions fixture (details and methodology in the
gist above):

| build | fixture total |
|---|---|
| `--features cuda` | 349 ms |
| `--features flash-attn` | **135 ms** |
| *reference: the original Python implementation, same GPU* | *165 ms* |
| *reference: the jev API this is benchmarked against* | *441 ms* |

On the isolated forward pass at an identical `b=7, s=1024` batch, this port is 60.3 ms against
PyTorch's 63.1 ms (`examples/bench_fwd.rs` mirrors a PyTorch script for a like-for-like number).

Getting there was mostly about finding places where candle silently takes a slow path, which
`examples/bench_ops.rs` (per-op micro-benchmarks at the real layer shapes) and
`examples/bench_fwd.rs` exist to surface:

- `LayerNorm` only uses its fused CUDA kernel when a bias is present. ModernBERT's norms are
  bias-free, so they were taking an ~8-op fallback that upcasts to F32 — 18x slower than the
  memory traffic justifies (37.1 ms → 3.7 ms). Passing an explicit zero bias fixes it.
- `Linear` on a rank-3 input issues a *batched* GEMM instead of one large flattened GEMM (2.3x
  on the model's biggest matmul).
- RoPE and GeGLU were chains of 6 and 3 separate elementwise ops, each a full round trip through
  a 14.7 MB tensor, running several times over their bandwidth bound. `src/fused.rs` replaces
  each with one NVRTC-compiled CUDA kernel (22.8 ms → 2.3 ms and 15.5 ms → 4.4 ms), checked
  against the op chains they replace by unit tests.
- Scaling Q before the QK^T matmul rather than scaling the S×S scores after (~16x fewer
  elementwise ops, since `head_dim` << `seq_len`).
- Unpadding once for the whole encoder instead of gathering/scattering per layer.

Note that per-op timing via `LAYA_TIMING=1` inserts a `device.synchronize()` after each op, which
serializes otherwise-pipelined kernel launches and inflates what it measures — it's useful for
spotting *relative* outliers, but isolated benchmarks and end-to-end wall time are what the
numbers above are based on.

## Usage

```bash
# quick demo against a checkpoint directory (see laya.convaiinnovations.com for the weights)
laya /path/to/laya-typed-decisions

# ask a single choice question
laya ask --model-dir /path/to/laya --state "..." --question "..." --option "a" --option "b"

# answer a batch of typed questions against one state
laya answer input.json answers.json --model-dir /path/to/laya-typed-decisions

# answer a jev-questions-style batch file ({section: {state, questions}}) —
# scripts/jev_batch.py drives `laya answer` once per section
scripts/jev_batch.py questions.json answers.json --model-dir /path/to/laya-typed-decisions

# serve the Jev/Simple-Jev v1 protocol (POST /v1/classifier, GET /health, ...)
laya serve                                    # default checkpoint, 127.0.0.1:8000
laya serve --model /path/to/laya --port 9000 # explicit checkpoint + port

# RLCD training over a JSONL dataset
laya train /path/to/laya dataset.jsonl --epochs 3
```

Not affiliated with Convai Innovations; this is an independent reimplementation for the Rust
ecosystem. Model weights are not included in this repository — point the commands above at a
local checkpoint directory downloaded from Hugging Face
(`convaiinnovations/laya`, `-multilingual`, `-typed-decisions`).
