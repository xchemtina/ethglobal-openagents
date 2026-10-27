"""Read authenticated PoX scope through pinned upstream methods, without migration."""
from __future__ import annotations

import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import sqlite3
import stat
import sys
import tempfile


class BridgeError(Exception):
    """Stable public error code; do not embed source paths or private values."""


def strict_json(raw):
    def pairs(items):
        result = {}
        for key, value in items:
            if key in result:
                raise ValueError("duplicate_json_key")
            result[key] = value
        return result
    def constant(_):
        raise ValueError("nonfinite_json_number")
    try:
        return json.loads(raw.decode("utf-8"), object_pairs_hook=pairs,
                          parse_constant=constant)
    except (ValueError, UnicodeError, RecursionError):
        raise BridgeError("invalid_json") from None


def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"),
                      ensure_ascii=False, allow_nan=False).encode("utf-8")


def sha256(raw):
    return "sha256:" + hashlib.sha256(raw).hexdigest()


def read_regular(path, limit=1024 * 1024, *, secret=False):
    """No final symlink, special file, oversized read, or shared secret file."""
    try:
        fd = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(fd, "rb") as stream:
            info = os.fstat(stream.fileno())
            if not stat.S_ISREG(info.st_mode) or info.st_size > limit:
                raise BridgeError("invalid_file")
            if secret and (info.st_mode & 0o077 or info.st_uid != os.getuid()):
                raise BridgeError("credential_file_must_be_owner_only")
            raw = stream.read(limit + 1)
        if len(raw) > limit:
            raise BridgeError("file_too_large")
        return raw
    except OSError:
        raise BridgeError("file_unavailable") from None


def read_credential(path):
    try:
        credential = read_regular(path, 256, secret=True).decode("ascii").strip()
    except UnicodeError:
        raise BridgeError("credential_invalid") from None
    if not re.fullmatch(r"[a-f0-9]{64}", credential):
        raise BridgeError("credential_invalid")
    return credential


def validate_members(value):
    if (not isinstance(value, dict) or set(value) != {"schema", "source_instance", "members"}
            or value["schema"] != "pox.federation-members.v1"
            or not isinstance(value["source_instance"], str)
            or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.-]{0,39}", value["source_instance"])
            or not isinstance(value["members"], list) or not 3 <= len(value["members"]) <= 100):
        raise BridgeError("members_invalid")
    ids, names = set(), set()
    for member in value["members"]:
        if (not isinstance(member, dict) or set(member) != {"pox_staff_id", "federation_id"}
                or not isinstance(member["pox_staff_id"], str)
                or not re.fullmatch(r"staff_[a-f0-9]{24}", member["pox_staff_id"])
                or not isinstance(member["federation_id"], str)
                or not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_.-]{0,79}", member["federation_id"])
                or member["pox_staff_id"] in ids or member["federation_id"] in names):
            raise BridgeError("members_invalid")
        ids.add(member["pox_staff_id"])
        names.add(member["federation_id"])
    return value


class PoxSource:
    """One source per process, imported from an immutable verified temporary copy.

    No upstream code or private database is copied into the Git repository. The
    checked source is caller-installed and must match the compatibility manifest.
    No ambient sys.path entry or bytecode cache supplies the five source modules.
    """

    def __init__(self, source_path, database, members):
        self.members = validate_members(members)
        self.database = Path(database).absolute()
        if self.database.is_symlink() or not self.database.is_file():
            raise BridgeError("pox_database_missing_or_symlink")
        pins = strict_json(read_regular(Path(__file__).with_name("source-pins.json")))
        self.source_manifest_sha256 = sha256(canonical(pins))
        files = {}
        for name, expected in pins["files"].items():
            raw = read_regular(Path(source_path) / name)
            if hashlib.sha256(raw).hexdigest() != expected:
                raise BridgeError("pox_source_incompatible")
            files[name] = raw
        self._temporary = tempfile.TemporaryDirectory(prefix="pox-pinned-source-")
        snapshot = Path(self._temporary.name)
        for name, raw in files.items():
            (snapshot / name).write_bytes(raw)
        self._saved_modules = {}
        try:
            for name in ("network", "delivery", "measurements", "inquiry"):
                self._saved_modules[name] = sys.modules.get(name)
                module = importlib.util.module_from_spec(
                    importlib.util.spec_from_file_location(name, snapshot / (name + ".py")))
                sys.modules[name] = module
                exec(compile(files[name + ".py"], str(snapshot / (name + ".py")), "exec"), module.__dict__)
            self.module = sys.modules["inquiry"]
            # The upstream constructor creates/migrates schema. Never invoke it
            # for an actual user database. These read methods need only these fields.
            self.store = self.module.InquiryStore.__new__(self.module.InquiryStore)
            self.store.path = self.database
            self.store._admin_hash = None
            self.store._delivery_key = None
        except BaseException:
            self.close()
            raise

    def close(self):
        for name, module in self._saved_modules.items():
            if module is None:
                sys.modules.pop(name, None)
            else:
                sys.modules[name] = module
        self._saved_modules.clear()
        self._temporary.cleanup()

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()

    def scope(self, inquiry_id, credential):
        if not isinstance(inquiry_id, str) or not re.fullmatch(r"inq_[a-f0-9]{24}", inquiry_id):
            raise BridgeError("inquiry_invalid")
        try:
            connection = sqlite3.connect(self.database.as_uri() + "?mode=ro", uri=True, timeout=5)
            connection.row_factory = sqlite3.Row
            try:
                connection.execute("PRAGMA query_only=ON")
                connection.execute("BEGIN")
                if connection.execute("PRAGMA user_version").fetchone()[0] != 4:
                    raise BridgeError("pox_database_version_unsupported")
                row, actor = self.store._authorized(connection, inquiry_id, credential)
                handoff = connection.execute(
                    "SELECT id FROM handoffs WHERE inquiry_id=? ORDER BY version DESC LIMIT 1",
                    (inquiry_id,)).fetchone()
                if handoff is None:
                    raise BridgeError("pox_handoff_not_accepted")
                current = self.store._accepted_handoff(connection, row, handoff["id"])
                mapping = {x["pox_staff_id"]: x["federation_id"] for x in self.members["members"]}
                roles = ("coordinator", "producer", "reviewer")
                if any(current[role + "_id"] not in mapping for role in roles):
                    raise BridgeError("pox_member_not_mapped")
                scope = {
                    "scope_id": ":".join(("pox", self.members["source_instance"], inquiry_id, current["id"])),
                    "revision": current["scope_revision"],
                    "scope_sha256": "sha256:" + current["scope_digest"],
                    **{role + "_id": mapping[current[role + "_id"]] for role in roles},
                }
                return {
                    "schema": "pox.federation-scope.v1", "scope": scope,
                    "source_instance": self.members["source_instance"],
                    "inquiry_id": inquiry_id, "handoff_id": current["id"],
                    "inquiry_version": row["version"], "actor_id": actor["id"],
                    "source_manifest_sha256": self.source_manifest_sha256,
                    "members_sha256": sha256(canonical(self.members)),
                    "scientific_admission": False, "distribution_authorized": False,
                    "holder_delivery_authorized": False, "publication_authorized": False,
                    "trust_state": "local_snapshot",
                }
            finally:
                connection.close()
        except self.module.Error as error:
            raise BridgeError("pox_" + error.code) from None
        except sqlite3.Error:
            raise BridgeError("pox_database_unavailable") from None
