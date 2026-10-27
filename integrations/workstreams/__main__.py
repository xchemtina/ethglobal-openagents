"""Command-line access to a private local observer journal."""
from __future__ import annotations

import argparse
import json
import sys

from .journal import Journal
from .safeio import JournalError


def main(argv=None):
    parser = argparse.ArgumentParser(description=(
        "Local unsigned workstream observations and inbox delivery receipts. "
        "No execution, lease changes, scientific review or human acknowledgment."))
    parser.add_argument("--journal", required=True, help="Private local journal directory")
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("init", help="Create a new private journal; never overwrite")
    register = commands.add_parser("register", help="Create a recipient's delivery-only capability")
    register.add_argument("--recipient", required=True)
    register.add_argument("--token-file", required=True, help="New file in a separate private directory")
    observe = commands.add_parser("observe", help="Retain exact receipt bytes and delivery intent atomically")
    observe.add_argument("--receipt", required=True)
    observe.add_argument("--expect-sha256", required=True, help="Independently obtained sha256:<64 hex> digest")
    for option in ("workstream", "source-host", "source-session", "owner-reported", "next-step"):
        observe.add_argument("--" + option, required=True)
    observe.add_argument("--gate-reported", choices=("open", "paused", "unknown"), required=True)
    observe.add_argument("--recipient", action="append", required=True)
    for name in ("inbox", "received"):
        command = commands.add_parser(name)
        command.add_argument("--recipient", required=True)
        command.add_argument("--token-file", required=True)
        if name == "inbox":
            command.add_argument("--limit", type=int, default=100)
        else:
            command.add_argument("--event", required=True)
    commands.add_parser("audit", help="Detect local corruption; does not authenticate provenance")
    args = parser.parse_args(argv)
    try:
        journal = Journal.create(args.journal) if args.command == "init" else Journal(args.journal)
        if args.command == "init":
            result = {"status": "created", "mode": "observer_only"}
        elif args.command == "register":
            result = journal.register(args.recipient, args.token_file)
        elif args.command == "observe":
            result = journal.observe(
                args.receipt, args.expect_sha256, workstream_id=args.workstream,
                source_host=args.source_host, source_session=args.source_session,
                owner_last_reported=args.owner_reported, gate_reported=args.gate_reported,
                next_step_proposed=args.next_step, recipients=args.recipient)
        elif args.command == "inbox":
            result = journal.inbox(args.recipient, args.token_file, limit=args.limit)
        elif args.command == "received":
            result = journal.received(args.recipient, args.token_file, args.event)
        else:
            result = journal.audit()
        print(json.dumps(result, sort_keys=True))
        return 0
    except (JournalError, OSError) as error:
        message = str(error) if isinstance(error, JournalError) else "filesystem_unavailable"
        print(json.dumps({"error": message}), file=sys.stderr)
        return 2


if __name__ == "__main__":
    sys.exit(main())
