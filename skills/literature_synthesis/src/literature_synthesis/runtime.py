"""LLM runtime adapter.

Selects between four explicit runtimes plus an offline fixture mode:

* `mlx-local` (default) -- MLX via `mlx_vlm` with a local VLM checkpoint
  (Qwen3.6-35B-A3B-4bit by default). Temperature 0. Requires Apple Silicon
  and the optional `mlx-vlm` extra. The model + processor are cached on the
  runtime instance so a corpus pass only pays the load cost once.
* `local-ollama` -- HTTP call to a local Ollama server.
* `openrouter` -- HTTP call to OpenRouter (uses `OPENROUTER_API_KEY`).
* `openai` -- HTTP call to OpenAI (uses `OPENAI_API_KEY`).
* `offline` -- read a committed fixture JSON; used by tests and `--offline`.

Selection: the runtime is chosen by the `CHIMIACLAW_LITERATURE_RUNTIME`
environment variable (default: `mlx-local`). All runtimes return raw model
output as a string; parsing into the typed schema happens in `extract.py`.
"""

from __future__ import annotations

import json
import os
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Optional, Protocol

from .schema import LiteratureRuntime

DEFAULT_MLX_MODEL_DIR = Path("~/mlx-models/Qwen3.6-35B-A3B-4bit").expanduser()
DEFAULT_MLX_MODEL_ID = "Qwen3.6-35B-A3B-4bit"
DEFAULT_TEMPERATURE = 0.0
DEFAULT_SEED = 0
# The MolADT prompt is large (worked benzene example + schema) and the
# response carries structural molecule objects, so the token budget has to
# accommodate both. 4096 was empirically insufficient and clipped the model
# mid-preamble before any JSON appeared.
DEFAULT_MAX_TOKENS = 12288

ENV_RUNTIME = "CHIMIACLAW_LITERATURE_RUNTIME"
ENV_MLX_MODEL_DIR = "CHIMIACLAW_LITERATURE_MLX_MODEL_DIR"
ENV_OLLAMA_URL = "CHIMIACLAW_LITERATURE_OLLAMA_URL"
ENV_OFFLINE_FIXTURE = "CHIMIACLAW_LITERATURE_OFFLINE_FIXTURE"


class RuntimeError_(RuntimeError):
    """Raised when the configured runtime cannot fulfil a request."""


class GenerationRuntime(Protocol):
    """Common interface every runtime adapter implements."""

    runtime: LiteratureRuntime
    model_id: str
    model_path: Optional[str]

    def generate(self, prompt: str) -> str: ...


@dataclass
class OfflineRuntime:
    """Returns a committed fixture string. Used by tests and `--offline`."""

    fixture_path: Path
    runtime: LiteratureRuntime = LiteratureRuntime.mlx_local
    model_id: str = "fixture"
    model_path: Optional[str] = None

    def generate(self, prompt: str) -> str:  # noqa: ARG002 -- prompt unused on purpose
        if not self.fixture_path.exists():
            raise RuntimeError_(
                f"Offline runtime requires fixture at {self.fixture_path}"
            )
        return self.fixture_path.read_text(encoding="utf-8")


@dataclass
class MlxLocalRuntime:
    """MLX with a local VLM checkpoint, loaded via `mlx_vlm`.

    The model + processor are cached on the instance after the first call so a
    multi-paper corpus run only pays the multi-second load cost once.
    """

    runtime: LiteratureRuntime = LiteratureRuntime.mlx_local
    model_id: str = DEFAULT_MLX_MODEL_ID
    model_path: Optional[str] = str(DEFAULT_MLX_MODEL_DIR)
    temperature: float = DEFAULT_TEMPERATURE
    seed: int = DEFAULT_SEED
    max_tokens: int = DEFAULT_MAX_TOKENS
    _model: Any = field(default=None, init=False, repr=False, compare=False)
    _processor: Any = field(default=None, init=False, repr=False, compare=False)
    _config: Any = field(default=None, init=False, repr=False, compare=False)

    def _ensure_loaded(self) -> None:
        if self._model is not None:
            return
        try:
            from mlx_vlm import load  # type: ignore
            from mlx_vlm.utils import load_config  # type: ignore
        except Exception as exc:  # pragma: no cover -- env-specific
            raise RuntimeError_(
                "mlx-vlm is not installed. Run `uv pip install mlx-vlm` on "
                "Apple Silicon, or set CHIMIACLAW_LITERATURE_RUNTIME=offline."
            ) from exc
        if not Path(str(self.model_path)).expanduser().exists():
            raise RuntimeError_(
                f"MLX model directory {self.model_path} not found. Set "
                f"{ENV_MLX_MODEL_DIR} or place the model at the default path."
            )
        resolved = str(Path(str(self.model_path)).expanduser())
        model, processor = load(resolved)
        config = load_config(resolved)
        self._model = model
        self._processor = processor
        self._config = config

    def generate(self, prompt: str) -> str:
        try:
            from mlx_vlm import generate  # type: ignore
            from mlx_vlm.prompt_utils import apply_chat_template  # type: ignore
        except Exception as exc:  # pragma: no cover -- env-specific
            raise RuntimeError_(
                "mlx-vlm is not installed. Run `uv pip install mlx-vlm` on "
                "Apple Silicon, or set CHIMIACLAW_LITERATURE_RUNTIME=offline."
            ) from exc

        self._ensure_loaded()
        formatted = apply_chat_template(
            self._processor,
            self._config,
            prompt,
            num_images=0,
            num_audios=0,
        )
        result = generate(
            self._model,
            self._processor,
            formatted,
            image=None,
            audio=None,
            video=None,
            max_tokens=self.max_tokens,
            temperature=self.temperature,
            verbose=False,
        )
        # mlx_vlm 0.5+ returns a GenerationResult dataclass; older versions
        # returned a plain string. Handle both.
        text = getattr(result, "text", None)
        return text if text is not None else str(result)


@dataclass
class OllamaRuntime:
    runtime: LiteratureRuntime = LiteratureRuntime.local_ollama
    model_id: str = "gpt-oss:120b"
    model_path: Optional[str] = None
    base_url: str = "http://localhost:11434"
    temperature: float = DEFAULT_TEMPERATURE
    seed: int = DEFAULT_SEED

    def generate(self, prompt: str) -> str:
        try:
            import httpx  # type: ignore
        except Exception as exc:  # pragma: no cover -- env-specific
            raise RuntimeError_("httpx is required for the Ollama runtime") from exc
        response = httpx.post(
            f"{self.base_url.rstrip('/')}/api/generate",
            json={
                "model": self.model_id,
                "prompt": prompt,
                "stream": False,
                "options": {"temperature": self.temperature, "seed": self.seed},
            },
            timeout=600.0,
        )
        response.raise_for_status()
        body = response.json()
        return body.get("response", "")


@dataclass
class OpenAIRuntime:
    runtime: LiteratureRuntime = LiteratureRuntime.openai
    model_id: str = "gpt-4o"
    model_path: Optional[str] = None
    temperature: float = DEFAULT_TEMPERATURE
    seed: int = DEFAULT_SEED

    def generate(self, prompt: str) -> str:
        try:
            from openai import OpenAI  # type: ignore
        except Exception as exc:  # pragma: no cover -- env-specific
            raise RuntimeError_(
                "openai package is required for the openai runtime"
            ) from exc
        client = OpenAI()
        response = client.chat.completions.create(
            model=self.model_id,
            temperature=self.temperature,
            seed=self.seed,
            messages=[{"role": "user", "content": prompt}],
        )
        return response.choices[0].message.content or ""


@dataclass
class OpenRouterRuntime:
    runtime: LiteratureRuntime = LiteratureRuntime.openrouter
    model_id: str = "openai/gpt-4o"
    model_path: Optional[str] = None
    temperature: float = DEFAULT_TEMPERATURE
    seed: int = DEFAULT_SEED

    def generate(self, prompt: str) -> str:
        try:
            import httpx  # type: ignore
        except Exception as exc:  # pragma: no cover -- env-specific
            raise RuntimeError_("httpx is required for the OpenRouter runtime") from exc
        api_key = os.environ.get("OPENROUTER_API_KEY")
        if not api_key:
            raise RuntimeError_("OPENROUTER_API_KEY is not set")
        response = httpx.post(
            "https://openrouter.ai/api/v1/chat/completions",
            headers={"Authorization": f"Bearer {api_key}"},
            json={
                "model": self.model_id,
                "temperature": self.temperature,
                "seed": self.seed,
                "messages": [{"role": "user", "content": prompt}],
            },
            timeout=600.0,
        )
        response.raise_for_status()
        body = response.json()
        return body["choices"][0]["message"]["content"]


def select_runtime(
    *,
    offline_fixture: Optional[Path] = None,
    forced: Optional[str] = None,
) -> GenerationRuntime:
    """Pick a runtime based on env, an explicit override, or offline mode."""
    raw = (forced or os.environ.get(ENV_RUNTIME, "mlx-local")).strip().lower()
    if raw == "offline":
        path = offline_fixture or Path(
            os.environ.get(ENV_OFFLINE_FIXTURE, "")
        ).expanduser()
        return OfflineRuntime(fixture_path=path)
    if raw == "mlx-local":
        model_dir = Path(
            os.environ.get(ENV_MLX_MODEL_DIR, str(DEFAULT_MLX_MODEL_DIR))
        ).expanduser()
        return MlxLocalRuntime(
            model_id=model_dir.name or DEFAULT_MLX_MODEL_ID,
            model_path=str(model_dir),
        )
    if raw == "local-ollama":
        return OllamaRuntime(
            base_url=os.environ.get(ENV_OLLAMA_URL, "http://localhost:11434")
        )
    if raw == "openai":
        return OpenAIRuntime()
    if raw == "openrouter":
        return OpenRouterRuntime()
    raise RuntimeError_(f"Unknown runtime {raw!r}. Set {ENV_RUNTIME} explicitly.")


def _strip_fences(text: str) -> str:
    text = text.strip()
    if text.startswith("```"):
        # Find the first newline after the fence and drop the language tag.
        first_nl = text.find("\n")
        if first_nl != -1:
            text = text[first_nl + 1 :]
        # Drop trailing fence if present.
        if text.rstrip().endswith("```"):
            text = text.rstrip()[: -len("```")]
    return text.strip()


def _iter_balanced_objects(text: str):
    """Yield every top-level ``{...}`` substring in order.

    Walks the text character by character tracking string-literal state and
    brace depth. When depth returns to zero, emits the span and continues
    scanning for the next opening brace. Unlike :func:`_find_balanced_object`
    (which only returned the first match), this lets the caller try multiple
    candidates when an upstream model interleaves example JSON blocks with
    its real answer.
    """
    depth = 0
    in_string = False
    escape = False
    start = -1
    for idx, ch in enumerate(text):
        if in_string:
            if escape:
                escape = False
            elif ch == "\\":
                escape = True
            elif ch == '"':
                in_string = False
            continue
        if ch == '"':
            in_string = True
            continue
        if ch == "{":
            if depth == 0:
                start = idx
            depth += 1
        elif ch == "}" and depth > 0:
            depth -= 1
            if depth == 0 and start != -1:
                yield text[start : idx + 1]
                start = -1


def _find_balanced_object(text: str) -> Optional[str]:
    """Return the first top-level ``{...}`` substring, or ``None``."""
    for span in _iter_balanced_objects(text):
        return span
    return None


def parse_json_object(raw: str) -> dict:
    r"""Return the JSON object emitted by the model.

    Handles four layers of common LLM noise:

    1. Triple-backtick markdown fences.
    2. Reasoning preambles emitted by thinking-style models (e.g. Qwen3-VL),
       where the JSON appears mid-response.
    3. Trailing chatter after the closing brace.
    4. Models that interleave example JSON blocks inside their reasoning
       before producing the real answer. In that case, the LAST top-level
       ``{...}`` substring that parses cleanly is taken as the answer; the
       earlier blocks are assumed to be examples or sketches.

    Raises ``ValueError`` with a snippet of the raw output when no JSON
    object can be recovered, so callers can persist the offending text for
    debugging.
    """
    stripped = _strip_fences(raw)
    try:
        return json.loads(stripped)
    except json.JSONDecodeError:
        pass

    last_parsed: Optional[dict] = None
    for span in _iter_balanced_objects(stripped):
        try:
            parsed = json.loads(span)
        except json.JSONDecodeError:
            continue
        if isinstance(parsed, dict):
            last_parsed = parsed
    if last_parsed is not None:
        return last_parsed

    snippet = raw.strip()[:600].replace("\n", " \\n ")
    raise ValueError(
        f"parse_json_object: no JSON object found in model output. "
        f"raw[:600]={snippet!r}"
    )


__all__ = [
    "DEFAULT_MLX_MODEL_DIR",
    "DEFAULT_MLX_MODEL_ID",
    "ENV_RUNTIME",
    "ENV_MLX_MODEL_DIR",
    "ENV_OLLAMA_URL",
    "ENV_OFFLINE_FIXTURE",
    "GenerationRuntime",
    "OfflineRuntime",
    "MlxLocalRuntime",
    "OllamaRuntime",
    "OpenAIRuntime",
    "OpenRouterRuntime",
    "RuntimeError_",
    "select_runtime",
    "parse_json_object",
]
