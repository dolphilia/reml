#!/usr/bin/env python3
import argparse
import os
import subprocess
import sys


def strip_yaml_front_matter(text: str) -> str:
    lines = text.splitlines()
    if not lines or lines[0].strip() != "---":
        return text
    for i in range(1, len(lines)):
        if lines[i].strip() in ("---", "..."):
            return "\n".join(lines[i + 1 :]).lstrip("\n")
    return text


def replace_mermaid_blocks(text: str, stem: str, image_rel: str):
    lines = text.splitlines()
    out = []
    blocks = []
    in_block = False
    current = []
    index = 0
    for line in lines:
        stripped = line.strip()
        if not in_block and stripped.startswith("```mermaid"):
            in_block = True
            current = []
            continue
        if in_block:
            if stripped.startswith("```"):
                index += 1
                code = "\n".join(current).rstrip() + "\n"
                blocks.append(code)
                name = f"{stem}-mermaid-{index:02d}.pdf"
                out.append(f"![]({image_rel}/{name})")
                in_block = False
            else:
                current.append(line)
            continue
        out.append(line)
    if in_block:
        raise ValueError("Unclosed mermaid code block")
    return "\n".join(out).rstrip() + "\n", blocks


def extract_mermaid_blocks(text: str):
    lines = text.splitlines()
    blocks = []
    in_block = False
    current = []
    for line in lines:
        stripped = line.strip()
        if not in_block and stripped.startswith("```mermaid"):
            in_block = True
            current = []
            continue
        if in_block:
            if stripped.startswith("```"):
                code = "\n".join(current).rstrip() + "\n"
                blocks.append(code)
                in_block = False
            else:
                current.append(line)
    if in_block:
        raise ValueError("Unclosed mermaid code block")
    return blocks


def list_drafts(paths):
    return [p for p in paths if os.path.basename(p) != "README.md"]


def run_images(args):
    drafts = list_drafts(args.drafts)
    os.makedirs(args.out_dir, exist_ok=True)
    for path in drafts:
        text = open(path, "r", encoding="utf-8").read()
        blocks = extract_mermaid_blocks(text)
        if not blocks:
            continue
        stem = os.path.splitext(os.path.basename(path))[0]
        for idx, code in enumerate(blocks, 1):
            base = f"{stem}-mermaid-{idx:02d}"
            mmd_path = os.path.join(args.out_dir, f"{base}.mmd")
            pdf_path = os.path.join(args.out_dir, f"{base}.pdf")
            with open(mmd_path, "w", encoding="utf-8") as f:
                f.write(code)
            cmd = [args.mmdc, "-i", mmd_path, "-o", pdf_path]
            if args.config_file:
                cmd.extend(["--configFile", args.config_file])
            if args.puppeteer_config:
                cmd.extend(["-p", args.puppeteer_config])
            subprocess.run(cmd, check=True)


def run_concat(args):
    drafts = list_drafts(args.drafts)
    parts = []
    if args.title or args.author or args.lang or args.date:
        front = ["---"]
        if args.title:
            front.append(f'title: "{args.title}"')
        if args.author:
            front.append(f'author: "{args.author}"')
        if args.lang:
            front.append(f"lang: {args.lang}")
        if args.date:
            front.append(f'date: "{args.date}"')
        front.append("---")
        parts.append("\n".join(front) + "\n")
    for path in drafts:
        text = open(path, "r", encoding="utf-8").read()
        text = strip_yaml_front_matter(text)
        stem = os.path.splitext(os.path.basename(path))[0]
        replaced, _ = replace_mermaid_blocks(text, stem, args.image_rel)
        parts.append(replaced.rstrip())
    os.makedirs(os.path.dirname(args.out_file), exist_ok=True)
    with open(args.out_file, "w", encoding="utf-8") as f:
        f.write("\n\n".join(parts).rstrip() + "\n")


def build_parser():
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)

    images = sub.add_parser("images")
    images.add_argument("--mmdc", default="mmdc")
    images.add_argument("--out-dir", required=True)
    images.add_argument("--config-file", default="")
    images.add_argument("--puppeteer-config", default="")
    images.add_argument("--drafts", nargs="+", required=True)
    images.set_defaults(func=run_images)

    concat = sub.add_parser("concat")
    concat.add_argument("--out-file", required=True)
    concat.add_argument("--image-rel", default="images")
    concat.add_argument("--title", default="")
    concat.add_argument("--author", default="")
    concat.add_argument("--lang", default="")
    concat.add_argument("--date", default="")
    concat.add_argument("--drafts", nargs="+", required=True)
    concat.set_defaults(func=run_concat)

    return parser


def main():
    parser = build_parser()
    args = parser.parse_args()
    try:
        args.func(args)
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
