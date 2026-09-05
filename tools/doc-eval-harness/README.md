# AutoVaxx Documentation Evaluation Harness

This package is a synthetic-only control harness for testing a future local documentation model. It
contains deterministic fake providers plus a loopback-only Ollama evaluation adapter and campaign CLI.
It has no patch-application command, product integration, PREIS access, cloud fallback, or PHI authority.

From this directory:

```sh
uv sync --locked --dev
uv run pytest
uv run ruff check .
uv run mypy
uv audit --locked
```

The model approval file pins the installed model's full digest, GGUF parameter-size label, and hash of
its runtime license text. Do not change those values merely to make a probe pass. Start a developer-owned
service with cloud features disabled:

```sh
OLLAMA_NO_CLOUD=1 OLLAMA_HOST=127.0.0.1:11434 ollama serve
```

Run that service only for the synthetic campaign and stop it afterward. The local API is not an
application authentication boundary; other local processes may reach loopback, and a product browser
must never receive direct model-endpoint access.

Create an output directory outside this repository, then run a synthetic campaign:

```sh
mkdir -m 700 /private/tmp/autovaxx-doc-eval
uv run python -m autovaxx_doc_harness.campaign \
  --approvals evaluation/model-approvals.json \
  --fixtures evaluation/cases \
  --output-root /private/tmp/autovaxx-doc-eval \
  --model qwen2.5:3b \
  --iterations 50 \
  --deadline-seconds 60
```

The adapter always uses `http://127.0.0.1:11434`, ignores proxy environment variables, disables
redirects, sends structured schemas with deterministic decoding options, and explicitly unloads the
model after the campaign. Model pulls are intentionally not automated. The current Qwen and Llama
entries use custom licenses; review and approve a license before distributing a model with AutoVaxx.

The provider rejects more than 64 selectable candidates or 12,288 bytes of combined generation input
before transport. These pilot limits bound work; they do not replace tokenizer-aware context checks.

The full authorization and security boundary is in
`../../docs/LOCAL_DOCUMENTATION_HARNESS_IMPLEMENTATION_CONTRACT.md`.
