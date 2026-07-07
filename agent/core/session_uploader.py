#!/usr/bin/env python3
"""
Stub — upload functionality is removed.
"""

import argparse
import sys

from dotenv import load_dotenv
load_dotenv()


def upload_session_as_file(
    session_file: str,
    repo_id: str,
    max_retries: int = 3,
    format: str = "row",
    token_env: str | None = None,
    private: bool = False,
) -> bool:
    """Upload is disabled — session is saved locally only."""
    return True


def retry_failed_uploads(
    directory: str,
    repo_id: str,
    format: str = "row",
    token_env: str | None = None,
    private: bool = False,
):
    pass


def _str2bool(v: str) -> bool:
    return str(v).strip().lower() in {"1", "true", "yes", "on"}


if __name__ == "__main__":
    parser = argparse.ArgumentParser(prog="session_uploader.py")
    sub = parser.add_subparsers(dest="command", required=True)

    p_upload = sub.add_parser("upload")
    p_upload.add_argument("session_file")
    p_upload.add_argument("repo_id")
    p_upload.add_argument("--format", choices=["row", "claude_code"], default="row")
    p_upload.add_argument("--token-env", default=None)
    p_upload.add_argument("--private", default="false")

    p_retry = sub.add_parser("retry")
    p_retry.add_argument("directory")
    p_retry.add_argument("repo_id")
    p_retry.add_argument("--format", choices=["row", "claude_code"], default="row")
    p_retry.add_argument("--token-env", default=None)
    p_retry.add_argument("--private", default="false")

    args = parser.parse_args()

    if args.command == "upload":
        ok = upload_session_as_file(
            args.session_file,
            args.repo_id,
            format=args.format,
            token_env=args.token_env,
            private=_str2bool(args.private),
        )
        sys.exit(0 if ok else 1)

    if args.command == "retry":
        retry_failed_uploads(
            args.directory,
            args.repo_id,
            format=args.format,
            token_env=args.token_env,
            private=_str2bool(args.private),
        )
        sys.exit(0)

    parser.print_help()
    sys.exit(1)
