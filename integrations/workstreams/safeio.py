"""Small bounded file and JSON boundary for a locally owned Unix journal."""
from __future__ import annotations

import hashlib
import json
import math
import os
from pathlib import Path
import re
import stat

MAX_RECEIPT = 1024 * 1024
MAX_DATABASE = 128 * 1024 * 1024


class JournalError(ValueError):
    """A stable diagnostic that must not contain input bytes or credentials."""


def digest(raw: bytes) -> str:
    return "sha256:" + hashlib.sha256(raw).hexdigest()


def canonical(value) -> bytes:
    try:
        return json.dumps(value, sort_keys=True, separators=(",", ":"),
                          ensure_ascii=True, allow_nan=False).encode("utf-8")
    except (ValueError, TypeError, RecursionError):
        raise JournalError("invalid_json") from None


def strict_json(raw: bytes):
    if not isinstance(raw, bytes) or not raw or len(raw) > MAX_RECEIPT:
        raise JournalError("receipt_size_invalid")
    # Bound depth before invoking the recursive JSON decoder, respecting strings.
    depth = 0
    quoted = escaped = False
    for byte in raw:
        if quoted:
            if escaped:
                escaped = False
            elif byte == 92:
                escaped = True
            elif byte == 34:
                quoted = False
        elif byte == 34:
            quoted = True
        elif byte in (91, 123):
            depth += 1
            if depth > 32:
                raise JournalError("json_depth_exceeded")
        elif byte in (93, 125):
            depth -= 1

    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise JournalError("duplicate_json_key")
            result[key] = value
        return result

    def constant(_):
        raise JournalError("nonfinite_json_number")

    def integer(value):
        if len(value) > 64:
            raise JournalError("json_integer_too_large")
        return int(value)

    try:
        result = json.loads(raw.decode("utf-8"), object_pairs_hook=pairs,
                            parse_constant=constant, parse_int=integer)
    except (UnicodeError, ValueError, RecursionError) as error:
        if isinstance(error, JournalError):
            raise
        raise JournalError("invalid_json") from None
    if not isinstance(result, dict):
        raise JournalError("receipt_must_be_object")
    stack, count = [result], 0
    while stack:
        item = stack.pop()
        count += 1
        if count > 20000:
            raise JournalError("json_nodes_exceeded")
        if isinstance(item, dict):
            stack.extend(item.keys())
            stack.extend(item.values())
        elif isinstance(item, list):
            stack.extend(item)
        elif isinstance(item, float) and not math.isfinite(item):
            raise JournalError("nonfinite_json_number")
        elif isinstance(item, str):
            if len(item) > 65536 or any(0xD800 <= ord(c) <= 0xDFFF for c in item):
                raise JournalError("json_string_invalid")
    return result


def identifier(value: str) -> str:
    if not isinstance(value, str) or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.:-]{0,127}", value):
        raise JournalError("invalid_identifier")
    return value


def text_field(value: str, maximum=2048) -> str:
    if (not isinstance(value, str) or len(value) > maximum
            or any(ord(c) < 32 or 0xD800 <= ord(c) <= 0xDFFF for c in value)):
        raise JournalError("invalid_text_field")
    return value


def safe_path(path) -> Path:
    if os.name != "posix" or not hasattr(os, "O_NOFOLLOW"):
        raise JournalError("unix_permissions_required")
    raw = Path(path)
    if ".." in raw.parts:
        raise JournalError("parent_path_forbidden")
    path = Path(os.path.abspath(raw))
    for part in reversed([path, *path.parents]):
        try:
            info = part.lstat()
        except FileNotFoundError:
            continue
        except OSError:
            raise JournalError("path_unavailable") from None
        if stat.S_ISLNK(info.st_mode):
            raise JournalError("symlink_path_forbidden")
    return path


def private_directory(path) -> Path:
    path = safe_path(path)
    try:
        info = path.lstat()
    except OSError:
        raise JournalError("journal_directory_unavailable") from None
    if (not stat.S_ISDIR(info.st_mode) or info.st_uid != os.getuid()
            or stat.S_IMODE(info.st_mode) != 0o700):
        raise JournalError("journal_directory_not_private")
    return path


def check_file(info, maximum, private):
    if (not stat.S_ISREG(info.st_mode) or info.st_nlink != 1
            or info.st_uid != os.getuid() or info.st_size > maximum):
        raise JournalError("unsafe_or_oversized_file")
    if private and stat.S_IMODE(info.st_mode) != 0o600:
        raise JournalError("file_not_private")


def read_regular(path, maximum=MAX_RECEIPT, *, private=False) -> bytes:
    path = safe_path(path)
    try:
        before = path.lstat()
        check_file(before, maximum, private)
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(fd, "rb") as stream:
            opened = os.fstat(stream.fileno())
            check_file(opened, maximum, private)
            if (before.st_dev, before.st_ino) != (opened.st_dev, opened.st_ino):
                raise JournalError("input_changed")
            raw = stream.read(maximum + 1)
            after = os.fstat(stream.fileno())
            if (opened.st_size, opened.st_mtime_ns) != (after.st_size, after.st_mtime_ns):
                raise JournalError("input_changed")
    except OSError:
        raise JournalError("input_unavailable") from None
    if len(raw) > maximum:
        raise JournalError("input_too_large")
    return raw


def sync_directory(path):
    fd = os.open(path, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def write_new(path, raw: bytes):
    path = safe_path(path)
    private_directory(path.parent)
    try:
        fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        with os.fdopen(fd, "wb") as stream:
            stream.write(raw)
            stream.flush()
            os.fsync(stream.fileno())
        sync_directory(path.parent)
    except OSError:
        raise JournalError("output_exists_or_unavailable") from None
